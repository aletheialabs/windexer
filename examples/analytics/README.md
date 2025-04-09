# Solana Transaction Analytics

This project provides tools for analyzing Solana transaction data stored in Parquet format. It uses DuckDB for efficient data processing and Dash for interactive visualization.

## Features

- Real-time analysis of transaction data
- Interactive dashboard with visualizations
- Account activity analysis
- Transaction success rate tracking
- Fee analysis
- Time-based transaction volume analysis

## Prerequisites

- Python 3.8+
- pip or conda

## Installation

1. Create a virtual environment (recommended):
```bash
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
```

2. Install dependencies:
```bash
pip install -r requirements.txt
```

## Usage

1. Ensure you have Parquet files in the `data/exports` directory from the real-time indexer.

2. Run the analysis script:
```bash
python analyze_transactions.py
```

3. Open your web browser and navigate to:
```
http://localhost:8050
```

## Dashboard Features

The dashboard provides:

1. **Overall Statistics**
   - Total number of transactions
   - Success rate
   - Average transaction fee

2. **Transaction Volume Over Time**
   - Hourly transaction count
   - Interactive time series plot

3. **Success Rate Analysis**
   - Hourly success rate trends
   - Transaction success patterns

4. **Account Activity**
   - Top 20 most active accounts
   - Transaction count per account
   - Success rate per account

## Data Analysis

The script performs several analyses:

1. **Transaction Metrics**
   - Total transaction count
   - Average fees
   - Success rates
   - Slot coverage

2. **Time-based Analysis**
   - Hourly transaction volume
   - Success rate trends
   - Fee trends

3. **Account Analysis**
   - Most active accounts
   - Account success rates
   - Slot participation

## Customization

You can modify the analysis by:

1. Editing SQL queries in the `analyze_transaction_metrics` and `analyze_account_activity` functions
2. Adding new visualizations in the `create_dashboard` function
3. Adjusting the CSS styles in `assets/styles.css`

## Troubleshooting

If you encounter issues:

1. Ensure the Parquet files are in the correct directory
2. Check that all dependencies are installed
3. Verify the database connection
4. Check the console for error messages

## Contributing

Feel free to submit issues and enhancement requests! 