#!/bin/bash

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python -m venv --copies venv
fi

# Activate virtual environment
source venv/bin/activate

# Install dependencies
echo "Installing dependencies..."
pip install -r requirements.txt

# Verify installation
echo "Verifying installation..."
python -c "import polars; import plotly; import dash; import numpy; print('All dependencies installed successfully!')"

echo "Setup complete! You can now run:"
echo "source venv/bin/activate && python analyze_transactions.py" 