=== SWEEP SUMMARY ===
Sweeps completed: 500
Total bytes read: 1319965
Total time: 9.722798 seconds
Average time per sweep: 0.019446 seconds
Throughput: 132.58 KB/s

The sweep function is very fast, a reading every 0.02 seconds on average. However, the data only changes when the device competes a new sweep every 200-300ms or so, so we're reading the same data over and over again. 