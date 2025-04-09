#!/usr/bin/fish

# Create virtual environment if it doesn't exist
if not test -d venv
    echo "Creating virtual environment..."
    python -m venv --copies venv
end

# Activate virtual environment
source venv/bin/activate.fish

# Ensure we're using the virtual environment's Python
set -gx PATH (pwd)/venv/bin $PATH

# Install dependencies
echo "Installing dependencies..."
pip install -r requirements.txt

# Verify installation
echo "Verifying installation..."
python -c "import polars; import plotly; import dash; import numpy; print('All dependencies installed successfully!')"

echo "Setup complete! You can now run:"
echo "source venv/bin/activate.fish && python analyze_transactions.py" 