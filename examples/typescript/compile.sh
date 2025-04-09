#!/bin/bash
echo "Compiling TypeScript files..."

# Make sure the dist directory exists
mkdir -p dist

# Use the local tsc executable if it exists, otherwise try the global one
if [ -f "./node_modules/.bin/tsc" ]; then
  echo "Using local TypeScript compiler"
  ./node_modules/.bin/tsc
elif command -v tsc &> /dev/null; then
  echo "Using global TypeScript compiler"
  tsc
else
  echo "TypeScript compiler not found. Please install it with 'npm install -g typescript'"
  exit 1
fi

echo "Copying package.json to dist directory"
cp package.json dist/

echo "Compilation complete. Run with one of the following commands:"
echo "  node dist/query-all-data.js              # Standard output"
echo "  node dist/query-all-data.js --interactive # Interactive mode"
echo "  node dist/query-all-data.js --json        # JSON output (compact)"
echo "  node dist/query-all-data.js --pretty-json # JSON output (pretty-printed)" 