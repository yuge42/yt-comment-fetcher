#!/usr/bin/env bash

jq -r '
  .items[]?
  | (
      .author_details.display_name // "unknown"
    ) as $name
  | (
      .snippet.display_message // ""
    ) as $msg
  | "\u001b[36m[\($name)]\u001b[0m \($msg)"
'
