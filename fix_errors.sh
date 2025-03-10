#!/bin/bash
# Fixing ambiguous module errors
if [ -d "crates/windexer-common/src/errors" ]; then
  echo "Removing errors directory to fix ambiguity"
  rm -rf crates/windexer-common/src/errors
fi

# Make sure errors.rs exists
echo "Ensuring errors.rs exists"
touch crates/windexer-common/src/errors.rs 