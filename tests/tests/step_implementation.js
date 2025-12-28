/* globals gauge, step, beforeScenario, afterScenario */
'use strict';

const { spawn } = require('child_process');
const assert = require('assert');
const path = require('path');

// Configuration constants
const STARTUP_TIMEOUT_MS = 2000;
const MAX_WAIT_TIME_MS = 10000;
const CHECK_INTERVAL_MS = 500;
const MIN_MESSAGES_FOR_AUTO_RESOLVE = 5;
const SHUTDOWN_GRACE_PERIOD_MS = 1000;

// Helper functions to access scenario data store
function getStore() {
  return gauge.dataStore.scenarioStore;
}

function setFetcherProcess(process) {
  getStore().put('fetcherProcess', process);
}

function getFetcherProcess() {
  return getStore().get('fetcherProcess');
}

function setReceivedLines(lines) {
  getStore().put('receivedLines', lines);
}

function getReceivedLines() {
  return getStore().get('receivedLines') || [];
}

function setServerAddress(address) {
  getStore().put('serverAddress', address);
}

function getServerAddress() {
  return getStore().get('serverAddress');
}

function setApiKeyPath(path) {
  getStore().put('apiKeyPath', path);
}

function getApiKeyPath() {
  return getStore().get('apiKeyPath');
}

// Initialize scenario store before each scenario
beforeScenario(async function() {
  setReceivedLines([]);
  setFetcherProcess(null);
  setServerAddress(null);
  setApiKeyPath(null);
});

// Cleanup after each scenario
afterScenario(async function() {
  const fetcherProcess = getFetcherProcess();
  if (fetcherProcess && !fetcherProcess.killed) {
    console.log('Cleaning up fetcher process in afterScenario...');
    fetcherProcess.kill('SIGTERM');
    await new Promise(resolve => setTimeout(resolve, SHUTDOWN_GRACE_PERIOD_MS));
    if (!fetcherProcess.killed) {
      fetcherProcess.kill('SIGKILL');
    }
  }
});

// Store server address from environment or default
step('Server address from environment variable <envVar> or default <defaultAddress>', async function (envVar, defaultAddress) {
  const serverAddress = process.env[envVar] || defaultAddress;
  setServerAddress(serverAddress);
  console.log(`Server address set to: ${serverAddress}`);
});

// Store API key path from environment
step('API key path from environment variable <envVar>', async function (envVar) {
  const apiKeyPath = process.env[envVar];
  if (apiKeyPath) {
    setApiKeyPath(apiKeyPath);
    console.log(`API key path set to: ${apiKeyPath}`);
  } else {
    console.log(`Environment variable ${envVar} not set, no API key will be used`);
  }
});

// Start the fetcher application
step('Start the fetcher application', async function () {
  return new Promise((resolve, reject) => {
    setReceivedLines([]);
    
    // Determine the binary path - assuming it's built in target/debug or target/release
    const binaryPath = process.env.FETCHER_BINARY || path.join(__dirname, '../../target/debug/yt-comment-fetcher');
    
    console.log(`Starting fetcher from: ${binaryPath}`);
    
    // Spawn the fetcher process
    // The fetcher reads SERVER_ADDRESS from environment if we want to pass it
    const env = Object.assign({}, process.env);
    const serverAddress = getServerAddress();
    if (serverAddress) {
      env.SERVER_ADDRESS = serverAddress;
    }
    
    // Pass the video ID as named argument
    const args = ['--video-id', 'test-video-1'];
    
    // Add API key path if available
    const apiKeyPath = getApiKeyPath();
    if (apiKeyPath) {
      args.push('--api-key-path', apiKeyPath);
      console.log(`Using API key from: ${apiKeyPath}`);
    }
    
    const fetcherProcess = spawn(binaryPath, args, {
      env: env
    });
    
    setFetcherProcess(fetcherProcess);

    let startupTimeout = setTimeout(() => {
      console.log('Fetcher started (timeout reached)');
      resolve();
    }, STARTUP_TIMEOUT_MS);

    fetcherProcess.stdout.on('data', (data) => {
      const lines = data.toString().split('\n').filter(line => line.trim().length > 0);
      const receivedLines = getReceivedLines();
      lines.forEach(line => {
        console.log(`Fetcher stdout: ${line}`);
        receivedLines.push(line);
      });
      setReceivedLines(receivedLines);
      
      // Once we start receiving data, resolve the startup promise
      if (receivedLines.length > 0 && startupTimeout) {
        clearTimeout(startupTimeout);
        startupTimeout = null;
        resolve();
      }
    });

    fetcherProcess.stderr.on('data', (data) => {
      const output = data.toString();
      console.log(`Fetcher stderr: ${output}`);
    });

    fetcherProcess.on('error', (error) => {
      console.error(`Failed to start fetcher: ${error.message}`);
      if (startupTimeout) {
        clearTimeout(startupTimeout);
        startupTimeout = null;
      }
      reject(new Error(`Failed to start fetcher: ${error.message}`));
    });

    fetcherProcess.on('close', (code) => {
      console.log(`Fetcher process exited with code ${code}`);
    });
  });
});

// Wait for fetcher to connect and receive messages
step('Wait for fetcher to connect and receive messages', async function () {
  return new Promise((resolve) => {
    // Wait for messages to be received or timeout
    let elapsedTime = 0;
    
    const checkMessages = setInterval(() => {
      elapsedTime += CHECK_INTERVAL_MS;
      const receivedLines = getReceivedLines();
      
      if (receivedLines.length >= MIN_MESSAGES_FOR_AUTO_RESOLVE || elapsedTime >= MAX_WAIT_TIME_MS) {
        clearInterval(checkMessages);
        console.log(`Received ${receivedLines.length} lines after ${elapsedTime}ms`);
        resolve();
      }
    }, CHECK_INTERVAL_MS);
  });
});

// Verify fetcher outputs valid JSON stream
step('Verify fetcher outputs valid JSON stream', async function () {
  const receivedLines = getReceivedLines();
  assert.ok(receivedLines.length > 0, 'No output received from fetcher');
  
  receivedLines.forEach((line, index) => {
    try {
      const parsed = JSON.parse(line);
      console.log(`Line ${index} is valid JSON with kind: ${parsed.kind || 'N/A'}`);
    } catch (error) {
      throw new Error(`Line ${index} is not valid JSON: ${line}\nError: ${error.message}`);
    }
  });
  
  console.log(`Verified all ${receivedLines.length} lines are valid JSON`);
});

// Verify each JSON line has kind
step('Verify each JSON line has kind <kind>', async function (kind) {
  const receivedLines = getReceivedLines();
  receivedLines.forEach((line, index) => {
    const message = JSON.parse(line);
    assert.strictEqual(
      message.kind,
      kind,
      `Line ${index} has kind '${message.kind}' but expected '${kind}'`
    );
  });
  console.log(`Verified all ${receivedLines.length} lines have kind: ${kind}`);
});

// Verify each JSON line has author details
step('Verify each JSON line has author details in items', async function () {
  const receivedLines = getReceivedLines();
  receivedLines.forEach((line, index) => {
    const message = JSON.parse(line);
    assert.ok(message.items, `Line ${index} has no items field`);
    assert.ok(Array.isArray(message.items), `Line ${index} items is not an array`);
    assert.ok(message.items.length > 0, `Line ${index} has empty items array`);
    
    message.items.forEach((item, itemIndex) => {
      assert.ok(
        item.author_details,
        `Item ${itemIndex} in line ${index} has no author details`
      );
      assert.ok(
        item.author_details.display_name,
        `Item ${itemIndex} in line ${index} has no display name`
      );
    });
  });
  console.log('Verified all messages have author details');
});

// Verify minimum number of messages received
step('Verify received at least <count> JSON messages', async function (count) {
  const receivedLines = getReceivedLines();
  const expectedCount = parseInt(count, 10);
  assert.ok(
    receivedLines.length >= expectedCount,
    `Expected at least ${expectedCount} messages but received ${receivedLines.length}`
  );
  console.log(`Verified received at least ${expectedCount} messages (actual: ${receivedLines.length})`);
});

// Stop the fetcher application
step('Stop the fetcher application', async function () {
  const fetcherProcess = getFetcherProcess();
  if (fetcherProcess) {
    console.log('Stopping fetcher process...');
    fetcherProcess.kill('SIGTERM');
    
    // Wait for graceful shutdown
    await new Promise(resolve => setTimeout(resolve, SHUTDOWN_GRACE_PERIOD_MS));
    
    // Force kill if still running
    if (!fetcherProcess.killed) {
      fetcherProcess.kill('SIGKILL');
    }
    
    setFetcherProcess(null);
    console.log('Fetcher process stopped');
  }
});

// Start the fetcher application without video ID argument
step('Start the fetcher application without video ID argument', async function () {
  return new Promise((resolve, reject) => {
    setReceivedLines([]);
    
    const binaryPath = process.env.FETCHER_BINARY || path.join(__dirname, '../../target/debug/yt-comment-fetcher');
    
    console.log(`Starting fetcher without video ID from: ${binaryPath}`);
    
    const env = Object.assign({}, process.env);
    const serverAddress = getServerAddress();
    if (serverAddress) {
      env.SERVER_ADDRESS = serverAddress;
    }
    
    // No arguments - should fail
    const args = [];
    
    const fetcherProcess = spawn(binaryPath, args, {
      env: env
    });
    
    setFetcherProcess(fetcherProcess);

    let exitCode = null;
    let errorOutput = '';

    fetcherProcess.stdout.on('data', (data) => {
      console.log(`Fetcher stdout: ${data}`);
    });

    fetcherProcess.stderr.on('data', (data) => {
      const output = data.toString();
      console.log(`Fetcher stderr: ${output}`);
      errorOutput += output;
    });

    fetcherProcess.on('close', (code) => {
      exitCode = code;
      console.log(`Fetcher process exited with code ${code}`);
      getStore().put('exitCode', exitCode);
      getStore().put('errorOutput', errorOutput);
      resolve();
    });

    fetcherProcess.on('error', (error) => {
      console.error(`Failed to start fetcher: ${error.message}`);
      reject(new Error(`Failed to start fetcher: ${error.message}`));
    });
  });
});

// Start the fetcher application with invalid video ID
step('Start the fetcher application with invalid video ID <videoId>', async function (videoId) {
  return new Promise((resolve, reject) => {
    setReceivedLines([]);
    
    const binaryPath = process.env.FETCHER_BINARY || path.join(__dirname, '../../target/debug/yt-comment-fetcher');
    
    console.log(`Starting fetcher with invalid video ID from: ${binaryPath}`);
    
    const env = Object.assign({}, process.env);
    const serverAddress = getServerAddress();
    if (serverAddress) {
      env.SERVER_ADDRESS = serverAddress;
    }
    
    const args = ['--video-id', videoId];
    
    // Add API key path if available (needed when auth is enabled)
    const apiKeyPath = getApiKeyPath();
    if (apiKeyPath) {
      args.push('--api-key-path', apiKeyPath);
      console.log(`Using API key from: ${apiKeyPath}`);
    }
    
    const fetcherProcess = spawn(binaryPath, args, {
      env: env
    });
    
    setFetcherProcess(fetcherProcess);

    let errorOutput = '';

    fetcherProcess.stdout.on('data', (data) => {
      console.log(`Fetcher stdout: ${data}`);
    });

    fetcherProcess.stderr.on('data', (data) => {
      const output = data.toString();
      console.log(`Fetcher stderr: ${output}`);
      errorOutput += output;
    });

    fetcherProcess.on('close', (code) => {
      console.log(`Fetcher process exited with code ${code}`);
      getStore().put('exitCode', code);
      getStore().put('errorOutput', errorOutput);
      resolve();
    });

    fetcherProcess.on('error', (error) => {
      console.error(`Failed to start fetcher: ${error.message}`);
      reject(new Error(`Failed to start fetcher: ${error.message}`));
    });
    
    // Give it time to start and fail
    setTimeout(resolve, 3000);
  });
});

// Wait for fetcher to attempt connection
step('Wait for fetcher to attempt connection', async function () {
  // Wait a bit for the fetcher to try to connect and fail
  await new Promise(resolve => setTimeout(resolve, 2000));
});

// Verify fetcher exits with error about missing argument
step('Verify fetcher exits with error about missing argument', async function () {
  const exitCode = getStore().get('exitCode');
  const errorOutput = getStore().get('errorOutput');
  
  assert.ok(exitCode !== 0, `Expected non-zero exit code but got ${exitCode}`);
  assert.ok(
    errorOutput.includes('required') || errorOutput.includes('--video-id'),
    `Expected error about missing video-id argument but got: ${errorOutput}`
  );
  
  console.log(`Verified exit code ${exitCode} and error message about missing argument`);
});

// Verify fetcher exits with error about video not found
step('Verify fetcher exits with error about video not found', async function () {
  const exitCode = getStore().get('exitCode');
  const errorOutput = getStore().get('errorOutput');
  
  assert.ok(exitCode !== 0, `Expected non-zero exit code but got ${exitCode}`);
  assert.ok(
    errorOutput.includes('No video found') || errorOutput.includes('not found'),
    `Expected error about video not found but got: ${errorOutput}`
  );
  
  console.log(`Verified exit code ${exitCode} and error message about video not found`);
});

// Start the fetcher application without API key
step('Start the fetcher application without API key', async function () {
  return new Promise((resolve, reject) => {
    setReceivedLines([]);
    
    const binaryPath = process.env.FETCHER_BINARY || path.join(__dirname, '../../target/debug/yt-comment-fetcher');
    
    console.log(`Starting fetcher without API key from: ${binaryPath}`);
    
    const env = Object.assign({}, process.env);
    const serverAddress = getServerAddress();
    if (serverAddress) {
      env.SERVER_ADDRESS = serverAddress;
    }
    
    // Pass the video ID but NOT the API key
    const args = ['--video-id', 'test-video-1'];
    
    const fetcherProcess = spawn(binaryPath, args, {
      env: env
    });
    
    setFetcherProcess(fetcherProcess);

    let errorOutput = '';

    fetcherProcess.stdout.on('data', (data) => {
      console.log(`Fetcher stdout: ${data}`);
    });

    fetcherProcess.stderr.on('data', (data) => {
      const output = data.toString();
      console.log(`Fetcher stderr: ${output}`);
      errorOutput += output;
    });

    fetcherProcess.on('close', (code) => {
      console.log(`Fetcher process exited with code ${code}`);
      getStore().put('exitCode', code);
      getStore().put('errorOutput', errorOutput);
      resolve();
    });

    fetcherProcess.on('error', (error) => {
      console.error(`Failed to start fetcher: ${error.message}`);
      reject(new Error(`Failed to start fetcher: ${error.message}`));
    });
    
    // Give it time to start and fail
    setTimeout(resolve, 3000);
  });
});

// Verify fetcher exits with authentication error
step('Verify fetcher exits with authentication error', async function () {
  const exitCode = getStore().get('exitCode');
  const errorOutput = getStore().get('errorOutput');
  
  assert.ok(exitCode !== 0, `Expected non-zero exit code but got ${exitCode}`);
  assert.ok(
    errorOutput.includes('Unauthenticated') || 
    errorOutput.includes('401') || 
    errorOutput.includes('authentication') ||
    errorOutput.includes('status 401') ||
    errorOutput.toLowerCase().includes('unauthorized'),
    `Expected error about authentication but got: ${errorOutput}`
  );
  
  console.log(`Verified exit code ${exitCode} and error message about authentication`);
});

// Start the fetcher application with reconnect wait time
step('Start the fetcher application with reconnect wait time <waitSeconds> seconds', async function (waitSeconds) {
  return new Promise((resolve, reject) => {
    setReceivedLines([]);
    
    const binaryPath = process.env.FETCHER_BINARY || path.join(__dirname, '../../target/debug/yt-comment-fetcher');
    
    console.log(`Starting fetcher with reconnect wait time ${waitSeconds}s from: ${binaryPath}`);
    
    const env = Object.assign({}, process.env);
    const serverAddress = getServerAddress();
    if (serverAddress) {
      env.SERVER_ADDRESS = serverAddress;
    }
    
    // Pass the video ID and reconnect wait time
    const args = ['--video-id', 'test-video-1', '--reconnect-wait-secs', waitSeconds];
    
    // Add API key path if available
    const apiKeyPath = getApiKeyPath();
    if (apiKeyPath) {
      args.push('--api-key-path', apiKeyPath);
      console.log(`Using API key from: ${apiKeyPath}`);
    }
    
    const fetcherProcess = spawn(binaryPath, args, {
      env: env
    });
    
    setFetcherProcess(fetcherProcess);

    let startupTimeout = setTimeout(() => {
      console.log('Fetcher started (timeout reached)');
      resolve();
    }, STARTUP_TIMEOUT_MS);

    // Store all stderr output for later verification
    let stderrOutput = '';
    getStore().put('stderrOutput', stderrOutput);

    fetcherProcess.stdout.on('data', (data) => {
      const lines = data.toString().split('\n').filter(line => line.trim().length > 0);
      const receivedLines = getReceivedLines();
      lines.forEach(line => {
        console.log(`Fetcher stdout: ${line}`);
        receivedLines.push(line);
      });
      setReceivedLines(receivedLines);
      
      // Once we start receiving data, resolve the startup promise
      if (receivedLines.length > 0 && startupTimeout) {
        clearTimeout(startupTimeout);
        startupTimeout = null;
        resolve();
      }
    });

    fetcherProcess.stderr.on('data', (data) => {
      const output = data.toString();
      console.log(`Fetcher stderr: ${output}`);
      stderrOutput += output;
      getStore().put('stderrOutput', stderrOutput);
    });

    fetcherProcess.on('error', (error) => {
      console.error(`Failed to start fetcher: ${error.message}`);
      if (startupTimeout) {
        clearTimeout(startupTimeout);
        startupTimeout = null;
      }
      reject(new Error(`Failed to start fetcher: ${error.message}`));
    });

    fetcherProcess.on('close', (code) => {
      console.log(`Fetcher process exited with code ${code}`);
    });
  });
});

// Store initial message count
function setInitialMessageCount(count) {
  getStore().put('initialMessageCount', count);
}

function getInitialMessageCount() {
  return getStore().get('initialMessageCount') || 0;
}

// Store mock server process
function setMockServerProcess(process) {
  getStore().put('mockServerProcess', process);
}

function getMockServerProcess() {
  return getStore().get('mockServerProcess');
}

// Stop the mock server
step('Stop the mock server', async function () {
  console.log('Stopping mock server using docker compose...');
  
  const { execSync } = require('child_process');
  const projectRoot = path.join(__dirname, '../..');
  
  try {
    execSync('docker compose stop yt-api-mock', {
      cwd: projectRoot,
      stdio: 'inherit'
    });
    console.log('Mock server stopped');
  } catch (error) {
    console.error(`Failed to stop mock server: ${error.message}`);
    throw error;
  }
  
  // Store the message count before stopping
  const currentMessageCount = getReceivedLines().length;
  setInitialMessageCount(currentMessageCount);
  console.log(`Stored initial message count: ${currentMessageCount}`);
});

// Start the mock server
step('Start the mock server', async function () {
  console.log('Starting mock server using docker compose...');
  
  const { execSync } = require('child_process');
  const projectRoot = path.join(__dirname, '../..');
  
  try {
    execSync('docker compose start yt-api-mock', {
      cwd: projectRoot,
      stdio: 'inherit'
    });
    console.log('Mock server started');
    
    // Wait a bit for server to be fully ready
    await new Promise(resolve => setTimeout(resolve, 2000));
  } catch (error) {
    console.error(`Failed to start mock server: ${error.message}`);
    throw error;
  }
});

// Wait for a specific duration
step('Wait <seconds> seconds for connection to drop', async function (seconds) {
  const waitTime = parseInt(seconds, 10) * 1000;
  console.log(`Waiting ${seconds} seconds...`);
  await new Promise(resolve => setTimeout(resolve, waitTime));
});

// Verify fetcher logs connection error
step('Verify fetcher logs connection error', async function () {
  const stderrOutput = getStore().get('stderrOutput') || '';
  
  assert.ok(
    stderrOutput.includes('Error receiving message') || 
    stderrOutput.includes('Connection lost') ||
    stderrOutput.includes('Failed to connect') ||
    stderrOutput.includes('Failed to start stream'),
    `Expected connection error in logs but got: ${stderrOutput}`
  );
  
  console.log('Verified fetcher logged connection error');
});

// Verify fetcher logs reconnection attempt
step('Verify fetcher logs reconnection attempt', async function () {
  const stderrOutput = getStore().get('stderrOutput') || '';
  
  assert.ok(
    stderrOutput.includes('reconnecting') || 
    (stderrOutput.includes('Waiting') && stderrOutput.includes('seconds before reconnecting')),
    `Expected reconnection attempt in logs but got: ${stderrOutput}`
  );
  
  console.log('Verified fetcher logged reconnection attempt');
});

// Wait for fetcher to reconnect and receive messages
step('Wait for fetcher to reconnect and receive messages', async function () {
  const initialCount = getInitialMessageCount();
  console.log(`Waiting for new messages (initial count: ${initialCount})...`);
  
  return new Promise((resolve) => {
    let elapsedTime = 0;
    const maxWaitTime = 15000; // 15 seconds to allow for reconnection
    
    const checkMessages = setInterval(() => {
      elapsedTime += CHECK_INTERVAL_MS;
      const currentLines = getReceivedLines();
      
      // Check if we received new messages after reconnection
      if (currentLines.length > initialCount || elapsedTime >= maxWaitTime) {
        clearInterval(checkMessages);
        console.log(`Current message count: ${currentLines.length} (was: ${initialCount}) after ${elapsedTime}ms`);
        resolve();
      }
    }, CHECK_INTERVAL_MS);
  });
});

// Verify received new JSON messages after reconnection
step('Verify received new JSON messages after reconnection', async function () {
  const initialCount = getInitialMessageCount();
  const currentLines = getReceivedLines();
  
  assert.ok(
    currentLines.length > initialCount,
    `Expected more than ${initialCount} messages after reconnection but got ${currentLines.length}`
  );
  
  console.log(`Verified received new messages after reconnection (initial: ${initialCount}, now: ${currentLines.length})`);
});

// Verify reconnect wait time is logged
step('Verify reconnect wait time is <seconds> seconds in logs', async function (seconds) {
  const stderrOutput = getStore().get('stderrOutput') || '';
  
  assert.ok(
    stderrOutput.includes(`Reconnect wait time: ${seconds} seconds`) ||
    stderrOutput.includes(`Waiting ${seconds} seconds before reconnecting`),
    `Expected reconnect wait time of ${seconds} seconds in logs but got: ${stderrOutput}`
  );
  
  console.log(`Verified reconnect wait time of ${seconds} seconds in logs`);
});

// Start the fetcher application and expect failure
step('Start the fetcher application and expect failure', async function () {
  return new Promise((resolve, reject) => {
    setReceivedLines([]);
    
    const binaryPath = process.env.FETCHER_BINARY || path.join(__dirname, '../../target/debug/yt-comment-fetcher');
    
    console.log(`Starting fetcher expecting failure from: ${binaryPath}`);
    
    const env = Object.assign({}, process.env);
    const serverAddress = getServerAddress();
    if (serverAddress) {
      env.SERVER_ADDRESS = serverAddress;
    }
    
    // Pass the video ID
    const args = ['--video-id', 'test-video-1'];
    
    // Add API key path if available
    const apiKeyPath = getApiKeyPath();
    if (apiKeyPath) {
      args.push('--api-key-path', apiKeyPath);
      console.log(`Using API key from: ${apiKeyPath}`);
    }
    
    const fetcherProcess = spawn(binaryPath, args, {
      env: env
    });
    
    setFetcherProcess(fetcherProcess);

    let errorOutput = '';
    let stderrOutput = '';

    fetcherProcess.stdout.on('data', (data) => {
      console.log(`Fetcher stdout: ${data}`);
    });

    fetcherProcess.stderr.on('data', (data) => {
      const output = data.toString();
      console.log(`Fetcher stderr: ${output}`);
      errorOutput += output;
      stderrOutput += output;
    });

    fetcherProcess.on('close', (code) => {
      console.log(`Fetcher process exited with code ${code}`);
      getStore().put('exitCode', code);
      getStore().put('errorOutput', errorOutput);
      getStore().put('stderrOutput', stderrOutput);
      resolve();
    });

    fetcherProcess.on('error', (error) => {
      console.error(`Failed to start fetcher: ${error.message}`);
      reject(new Error(`Failed to start fetcher: ${error.message}`));
    });
    
    // Give it time to start and fail
    setTimeout(resolve, 3000);
  });
});

// Verify fetcher exits with connection error
step('Verify fetcher exits with connection error', async function () {
  const exitCode = getStore().get('exitCode');
  const errorOutput = getStore().get('errorOutput');
  
  assert.ok(exitCode !== 0, `Expected non-zero exit code but got ${exitCode}`);
  assert.ok(
    errorOutput.includes('Failed to connect') || 
    errorOutput.includes('connection') ||
    errorOutput.includes('Connection') ||
    errorOutput.toLowerCase().includes('error'),
    `Expected connection error but got: ${errorOutput}`
  );
  
  console.log(`Verified exit code ${exitCode} and connection error message`);
});

// Verify fetcher does not log reconnection attempts
step('Verify fetcher does not log reconnection attempts', async function () {
  const stderrOutput = getStore().get('stderrOutput') || '';
  
  assert.ok(
    !stderrOutput.includes('reconnecting') && 
    (!stderrOutput.includes('Waiting') || !stderrOutput.includes('seconds before reconnecting')),
    `Expected no reconnection attempts but got: ${stderrOutput}`
  );
  
  console.log('Verified fetcher did not log reconnection attempts');
});

// Record the current message count
step('Record the current message count', async function () {
  const currentCount = getReceivedLines().length;
  setInitialMessageCount(currentCount);
  console.log(`Recorded current message count: ${currentCount}`);
});

// Add new messages via mock control endpoint
step('Add new messages via mock control endpoint', async function () {
  console.log('Adding new messages via mock control endpoint...');
  
  const serverAddress = getServerAddress() || 'localhost:8081';
  // Extract host and port, removing https:// if present
  const controlAddress = serverAddress.replace(/^https?:\/\//, '').replace(':50051', ':8081').replace(':8080', ':8081');
  
  try {
    const http = require('http');
    const url = `http://${controlAddress}/control/add-messages?count=3`;
    
    console.log(`Calling control endpoint: ${url}`);
    
    await new Promise((resolve, reject) => {
      http.get(url, (res) => {
        let data = '';
        res.on('data', (chunk) => { data += chunk; });
        res.on('end', () => {
          console.log(`Control endpoint response: ${data}`);
          if (res.statusCode >= 200 && res.statusCode < 300) {
            resolve();
          } else {
            reject(new Error(`Control endpoint returned status ${res.statusCode}: ${data}`));
          }
        });
      }).on('error', (err) => {
        reject(err);
      });
    });
    
    console.log('Successfully added new messages via control endpoint');
  } catch (error) {
    console.error(`Failed to add messages via control endpoint: ${error.message}`);
    throw error;
  }
});

// Wait for fetcher to receive new messages
step('Wait for fetcher to receive new messages', async function () {
  const initialCount = getInitialMessageCount();
  console.log(`Waiting for new messages (initial count: ${initialCount})...`);
  
  return new Promise((resolve) => {
    let elapsedTime = 0;
    const maxWaitTime = 10000; // 10 seconds
    
    const checkMessages = setInterval(() => {
      elapsedTime += CHECK_INTERVAL_MS;
      const currentLines = getReceivedLines();
      
      // Check if we received new messages
      if (currentLines.length > initialCount || elapsedTime >= maxWaitTime) {
        clearInterval(checkMessages);
        console.log(`Current message count: ${currentLines.length} (was: ${initialCount}) after ${elapsedTime}ms`);
        resolve();
      }
    }, CHECK_INTERVAL_MS);
  });
});

// Verify fetcher received additional messages with correct pagination
step('Verify fetcher received additional messages with correct pagination', async function () {
  const initialCount = getInitialMessageCount();
  const currentLines = getReceivedLines();
  
  assert.ok(
    currentLines.length > initialCount,
    `Expected more than ${initialCount} messages after adding new ones but got ${currentLines.length}`
  );
  
  console.log(`Verified received additional messages (initial: ${initialCount}, now: ${currentLines.length})`);
  
  // Parse messages to check for duplicates by ID
  const messageIds = new Set();
  let duplicateFound = false;
  
  currentLines.forEach((line, index) => {
    try {
      const response = JSON.parse(line);
      if (response.items && Array.isArray(response.items)) {
        response.items.forEach((item) => {
          if (item.id) {
            if (messageIds.has(item.id)) {
              console.error(`Duplicate message ID found: ${item.id}`);
              duplicateFound = true;
            }
            messageIds.add(item.id);
          }
        });
      }
    } catch (error) {
      console.error(`Failed to parse line ${index}: ${error.message}`);
    }
  });
  
  assert.ok(!duplicateFound, 'Expected no duplicate message IDs (pagination should prevent duplicates)');
  console.log(`Verified no duplicate message IDs across ${messageIds.size} unique messages`);
});
