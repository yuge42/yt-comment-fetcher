/* globals gauge, step, beforeScenario */
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

let fetcherProcess = null;
let receivedLines = [];
let serverAddress = null;

// Store server address from environment or default
step('Server address from environment variable <envVar> or default <defaultAddress>', async function (envVar, defaultAddress) {
  serverAddress = process.env[envVar] || defaultAddress;
  console.log(`Server address set to: ${serverAddress}`);
});

// Start the fetcher application
step('Start the fetcher application', async function () {
  return new Promise((resolve, reject) => {
    receivedLines = [];
    
    // Determine the binary path - assuming it's built in target/debug or target/release
    const binaryPath = process.env.FETCHER_BINARY || path.join(__dirname, '../../target/debug/yt-comment-fetcher');
    
    console.log(`Starting fetcher from: ${binaryPath}`);
    
    // Spawn the fetcher process
    // The fetcher reads SERVER_ADDRESS from environment if we want to pass it
    const env = Object.assign({}, process.env);
    if (serverAddress) {
      env.SERVER_ADDRESS = serverAddress;
    }
    
    fetcherProcess = spawn(binaryPath, [], {
      env: env
    });

    let startupTimeout = setTimeout(() => {
      console.log('Fetcher started (timeout reached)');
      resolve();
    }, STARTUP_TIMEOUT_MS);

    fetcherProcess.stdout.on('data', (data) => {
      const lines = data.toString().split('\n').filter(line => line.trim().length > 0);
      lines.forEach(line => {
        console.log(`Fetcher stdout: ${line}`);
        receivedLines.push(line);
      });
      
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
  const expectedCount = parseInt(count, 10);
  assert.ok(
    receivedLines.length >= expectedCount,
    `Expected at least ${expectedCount} messages but received ${receivedLines.length}`
  );
  console.log(`Verified received at least ${expectedCount} messages (actual: ${receivedLines.length})`);
});

// Stop the fetcher application
step('Stop the fetcher application', async function () {
  if (fetcherProcess) {
    console.log('Stopping fetcher process...');
    fetcherProcess.kill('SIGTERM');
    
    // Wait for graceful shutdown
    await new Promise(resolve => setTimeout(resolve, SHUTDOWN_GRACE_PERIOD_MS));
    
    // Force kill if still running
    if (!fetcherProcess.killed) {
      fetcherProcess.kill('SIGKILL');
    }
    
    fetcherProcess = null;
    console.log('Fetcher process stopped');
  }
});
