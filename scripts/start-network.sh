# scripts/start-network.sh
#!/bin/bash

solana-test-validator --reset --log &
sleep 5  

for i in {0..2}; do
    gnome-terminal --tab --title "Node $i" -- \
        cargo run --example node -- \
            --index $i \
            --base-port 9000 \
            --enable-tip-route
    sleep 1
done

gnome-terminal --tab --title "Monitoring" -- watch -n 1 'curl -s http://localhost:9100/metrics'
