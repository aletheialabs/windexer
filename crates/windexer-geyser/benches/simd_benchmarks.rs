use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use agave_geyser_plugin_interface::geyser_plugin_interface::{
    ReplicaAccountInfo, ReplicaAccountInfoVersions, SlotStatus,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    transaction_status::{TransactionStatusMeta, TransactionStatus},
};
use windexer_geyser::simd_processing::SimdProcessor;

fn create_test_account(data_size: usize) -> (ReplicaAccountInfo<'static>, ReplicaAccountInfoVersions<'static>) {
    // Create static data for the test account
    let pubkey = Box::new([1u8; 32]);
    let owner = Box::new([2u8; 32]);
    let data = vec![3u8; data_size].into_boxed_slice();
    let lamports = 12345;
    let executable = false;
    let rent_epoch = 0;
    
    let account_info = ReplicaAccountInfo {
        pubkey: Box::leak(pubkey),
        owner: Box::leak(owner),
        lamports,
        data: Box::leak(data),
        executable,
        rent_epoch,
        write_version: 1,
    };
    
    let account = ReplicaAccountInfoVersions::V0_0_1(&account_info);
    (account_info, account)
}

fn bench_account_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("account_processing");
    let processor_simd = SimdProcessor::new(true).unwrap();
    let processor_no_simd = SimdProcessor::new(false).unwrap();
    
    // Test with different account data sizes
    for size in [64, 256, 1024, 4096].iter() {
        let (_, account) = create_test_account(*size);
        let slot = 42;
        
        group.bench_with_input(BenchmarkId::new("SIMD", size), size, |b, _| {
            b.iter(|| {
                black_box(processor_simd.process_account(account, slot).unwrap());
            });
        });
        
        group.bench_with_input(BenchmarkId::new("No SIMD", size), size, |b, _| {
            b.iter(|| {
                black_box(processor_no_simd.process_account(account, slot).unwrap());
            });
        });
    }
    group.finish();
}

fn bench_slot_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("slot_processing");
    let processor_simd = SimdProcessor::new(true).unwrap();
    let processor_no_simd = SimdProcessor::new(false).unwrap();
    
    let slot = 42;
    let parent = Some(41);
    let statuses = [SlotStatus::Processed, SlotStatus::Confirmed, SlotStatus::Rooted];
    
    for status in &statuses {
        group.bench_with_input(BenchmarkId::new("SIMD", format!("{:?}", status)), status, |b, _| {
            b.iter(|| {
                black_box(processor_simd.process_slot(slot, parent, *status).unwrap());
            });
        });
        
        group.bench_with_input(BenchmarkId::new("No SIMD", format!("{:?}", status)), status, |b, _| {
            b.iter(|| {
                black_box(processor_no_simd.process_slot(slot, parent, *status).unwrap());
            });
        });
    }
    group.finish();
}

fn bench_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");
    let processor_simd = SimdProcessor::new(true).unwrap();
    let processor_no_simd = SimdProcessor::new(false).unwrap();
    
    // Create a batch of accounts to process
    let batch_sizes = [10, 100, 1000];
    
    for &size in &batch_sizes {
        let mut accounts = Vec::with_capacity(size);
        for _ in 0..size {
            let (_, account) = create_test_account(64); // 64-byte data for each account
            accounts.push(account);
        }
        
        group.bench_with_input(BenchmarkId::new("SIMD", size), &size, |b, _| {
            b.iter(|| {
                for account in &accounts {
                    black_box(processor_simd.process_account(*account, 42).unwrap());
                }
            });
        });
        
        group.bench_with_input(BenchmarkId::new("No SIMD", size), &size, |b, _| {
            b.iter(|| {
                for account in &accounts {
                    black_box(processor_no_simd.process_account(*account, 42).unwrap());
                }
            });
        });
    }
    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    let processor_simd = SimdProcessor::new(true).unwrap();
    let processor_no_simd = SimdProcessor::new(false).unwrap();
    
    // Create a mixed workload of account updates and slot updates
    let (_, account) = create_test_account(256);
    let slot = 42;
    let parent = Some(41);
    let status = SlotStatus::Confirmed;
    
    group.bench_function("SIMD", |b| {
        b.iter(|| {
            // Process 5 accounts and 1 slot update
            for _ in 0..5 {
                black_box(processor_simd.process_account(account, slot).unwrap());
            }
            black_box(processor_simd.process_slot(slot, parent, status).unwrap());
        });
    });
    
    group.bench_function("No SIMD", |b| {
        b.iter(|| {
            // Process 5 accounts and 1 slot update
            for _ in 0..5 {
                black_box(processor_no_simd.process_account(account, slot).unwrap());
            }
            black_box(processor_no_simd.process_slot(slot, parent, status).unwrap());
        });
    });
    
    group.finish();
}

criterion_group!(
    benches, 
    bench_account_processing,
    bench_slot_processing,
    bench_batch_processing,
    bench_mixed_workload
);
criterion_main!(benches);
