import polars as pl
import plotly.graph_objects as go
from dash import Dash, html, dcc
import os
from datetime import datetime
import numpy as np
import duckdb

# Update the path to point to the correct exports directory
PARQUET_DIR = os.path.join(os.path.dirname(__file__), '..', '..', 'data', 'exports')

def load_transactions():
    """Load transaction data from parquet files"""
    parquet_files = [f for f in os.listdir(PARQUET_DIR) if f.endswith('.parquet')]
    if not parquet_files:
        raise ValueError(f"No parquet files found in {PARQUET_DIR}")
    
    print(f"Found {len(parquet_files)} parquet files")
    df = pl.concat([pl.read_parquet(os.path.join(PARQUET_DIR, f)) for f in parquet_files])
    return df

def analyze_transaction_metrics(df):
    """Analyze basic transaction metrics using Polars"""
    # Get overall statistics
    stats = df.select([
        pl.count().alias('total_transactions'),
        pl.col('fee').mean().alias('avg_fee'),
        pl.col('success').sum().alias('successful_transactions'),
        pl.col('slot').min().alias('first_slot'),
        pl.col('slot').max().alias('last_slot'),
        pl.col('slot').n_unique().alias('total_slots')
    ])
    
    # Get hourly transaction volume
    hourly_stats = df.with_columns(
        pl.from_epoch(pl.col('timestamp') / 1000).dt.truncate('1h').alias('hour')
    ).group_by('hour').agg([
        pl.count().alias('transaction_count'),
        pl.col('fee').mean().alias('avg_fee'),
        pl.col('success').sum().alias('successful_count')
    ]).sort('hour')
    
    return stats, hourly_stats

def analyze_account_activity(df):
    """Analyze account activity patterns using Polars"""
    # Explode accounts array and get most active accounts
    active_accounts = df.explode('accounts').group_by('accounts').agg([
        pl.count().alias('transaction_count'),
        pl.col('success').sum().alias('successful_transactions'),
        pl.col('slot').n_unique().alias('slots_participated')
    ]).sort('transaction_count', descending=True).head(100)
    
    return active_accounts

def create_dashboard(stats, hourly_stats, active_accounts):
    """Create an interactive dashboard using Dash"""
    app = Dash(__name__)
    
    # Convert Polars DataFrames to lists/dicts for Plotly
    stats_dict = stats.to_dicts()[0]
    hourly_data = hourly_stats.select(['hour', 'transaction_count', 'successful_count']).to_dicts()
    active_accounts_data = active_accounts.head(20).to_dicts()
    
    # Create figures
    fig_hourly = go.Figure(data=go.Scatter(
        x=[d['hour'] for d in hourly_data],
        y=[d['transaction_count'] for d in hourly_data],
        mode='lines',
        name='Transaction Count'
    ))
    fig_hourly.update_layout(title='Hourly Transaction Volume')
    
    fig_success_rate = go.Figure(data=go.Scatter(
        x=[d['hour'] for d in hourly_data],
        y=[d['successful_count'] for d in hourly_data],
        mode='lines',
        name='Successful Transactions'
    ))
    fig_success_rate.update_layout(title='Hourly Successful Transactions')
    
    fig_active_accounts = go.Figure(data=go.Bar(
        x=[d['accounts'] for d in active_accounts_data],
        y=[d['transaction_count'] for d in active_accounts_data]
    ))
    fig_active_accounts.update_layout(title='Top 20 Most Active Accounts')
    
    app.layout = html.Div([
        html.H1('Solana Transaction Analytics'),
        
        html.Div([
            html.H2('Overall Statistics'),
            html.Div([
                html.Div([
                    html.H3('Total Transactions'),
                    html.P(f"{stats_dict['total_transactions']:,}")
                ], className='stat-box'),
                html.Div([
                    html.H3('Success Rate'),
                    html.P(f"{(stats_dict['successful_transactions'] / stats_dict['total_transactions'] * 100):.2f}%")
                ], className='stat-box'),
                html.Div([
                    html.H3('Average Fee'),
                    html.P(f"{stats_dict['avg_fee']:,.0f} lamports")
                ], className='stat-box'),
            ], className='stats-container'),
            
            html.H2('Transaction Volume Over Time'),
            dcc.Graph(figure=fig_hourly),
            
            html.H2('Success Rate Over Time'),
            dcc.Graph(figure=fig_success_rate),
            
            html.H2('Most Active Accounts'),
            dcc.Graph(figure=fig_active_accounts),
        ])
    ])
    
    return app

def main():
    print("Loading transaction data...")
    df = load_transactions()
    
    print("Analyzing transaction metrics...")
    stats, hourly_stats = analyze_transaction_metrics(df)
    
    print("Analyzing account activity...")
    active_accounts = analyze_account_activity(df)
    
    print("Creating dashboard...")
    app = create_dashboard(stats, hourly_stats, active_accounts)
    
    print("Starting dashboard server...")
    app.run_server(debug=True, port=8050)

if __name__ == "__main__":
    main() 