"""
Benchmark script for pynanovnav2.
This matches the benchmark parameters from the pynanovna implementation.
"""

import sys
sys.path.append("d:\\Git repo\\pynanovnav2\\src")

from pynanovnav2.nanovnav2 import NanoVNAV2
import time
import statistics

def benchmark_readings(x, y, port="COM9"):
    """
    Benchmark the time taken to capture x readings, repeated y times.

    Args:
        x (int): Number of readings to capture in each trial.
        y (int): Number of times to repeat the benchmark.
        port (str): Serial port for NanoVNA V2.

    Returns:
        dict: Contains average time, standard deviation, and all trial times.
    """
    
    # Benchmark parameters matching the original
    start_freq = 50e3   # 50 kHz
    stop_freq = 900e6   # 900 MHz
    # Calculate step for 101 points: (900e6 - 50e3) / 100 = ~9 MHz
    step_freq = 8.9995e6  # ~9 MHz (gives 101 points)
    
    print(f"Benchmarking pynanovnav2 implementation")
    print(f"Port: {port}")
    print(f"Frequency range: {start_freq/1e3:.1f} kHz to {stop_freq/1e6:.1f} MHz")
    print(f"Step size: {step_freq/1e6:.1f} MHz (101 points)")
    print(f"Readings per trial: {x}")
    print(f"Number of trials: {y}")
    print("-" * 60)
    
    trial_times = []
    
    try:
        with NanoVNAV2(port, debug=False, useNumpy=True) as vna:
            # Get device info
            info = vna._get_id()
            print(f"Device: {info['title']}, Firmware: {info['firmware']}\n")
            
            # Configure sweep
            vna._set_sweep_range(start_freq, stop_freq, step_freq)
            
            for trial in range(y):
                print(f"Trial {trial + 1}/{y}...")
                start_time = time.time()
                
                # Capture x readings
                for reading in range(x):
                    data = vna._query_trace()
                    
                    # Print first frequency to match original benchmark output
                    if len(data['freq']) > 0:
                        print(f"  Reading {reading + 1}/{x}: Frequency: {data['freq'][0]} Hz, Points: {len(data['freq'])}")
                
                end_time = time.time()
                trial_time = end_time - start_time
                trial_times.append(trial_time)
                
                print(f"  Trial {trial + 1} completed in {trial_time:.3f} seconds\n")
            
            # Calculate statistics
            average_time = sum(trial_times) / len(trial_times)
            std_deviation = statistics.stdev(trial_times) if len(trial_times) > 1 else 0
            
            # Print results
            print("=" * 60)
            print("BENCHMARK RESULTS")
            print("=" * 60)
            print(f"Total readings captured: {x * y}")
            print(f"Average time per trial ({x} readings): {average_time:.3f} seconds")
            print(f"Average time per reading: {average_time/x:.3f} seconds")
            print(f"Standard deviation: {std_deviation:.3f} seconds")
            print(f"Readings per second: {x/average_time:.2f}")
            print("\nIndividual trial times:")
            for i, t in enumerate(trial_times, 1):
                print(f"  Trial {i}: {t:.3f} seconds")
            print("=" * 60)
            
            return {
                "average_time": average_time,
                "std_deviation": std_deviation,
                "trial_times": trial_times,
                "readings_per_second": x / average_time
            }
            
    except KeyboardInterrupt:
        print("\n\nBenchmark interrupted by user.")
        return None
    except Exception as e:
        print(f"\nError occurred: {e}")
        import traceback
        traceback.print_exc()
        return None

if __name__ == "__main__":
    # Set the number of readings per trial (x) and the number of trials (y)
    # These match the original benchmark parameters
    x = 10  # Number of readings per trial
    y = 1   # Number of trials
    
    # Update this to your NanoVNA V2 port
    PORT = "COM9"
    
    results = benchmark_readings(x, y, PORT)
    
    if results:
        print("\nBenchmark completed successfully!")
    else:
        print("\nBenchmark failed or was interrupted.")
