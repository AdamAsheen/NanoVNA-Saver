import time
import statistics
import pynanovna
import logging

# Suppress critical warnings about calibration
logging.basicConfig(level=logging.ERROR)

def benchmark_readings(x, y):
    """
    Benchmark the time taken to capture x readings, repeated y times.

    Args:
        x (int): Number of readings to capture in each trial.
        y (int): Number of times to repeat the benchmark.

    Returns:
        dict: Contains average time, standard deviation, and all trial times.
    """
    vna = pynanovna.VNA()
    vna.set_sweep(1.0e9, 1.4e9, 101)

    trial_times = []

    for trial in range(y):
        start_time = time.time()

        # Capture x readings
        stream = vna.stream()
        for _ in range(x):
            s11, s21, frequencies = next(stream)
            print(f"Frequency: {frequencies} Hz, S11: {s11}")

        end_time = time.time()
        trial_times.append(end_time - start_time)

    # Calculate statistics
    average_time_per_trial = sum(trial_times) / len(trial_times)
    average_time_per_reading = average_time_per_trial / x
    std_deviation = statistics.stdev(trial_times) if len(trial_times) > 1 else 0

    return {
        "average_time_per_trial": average_time_per_trial,
        "average_time_per_reading": average_time_per_reading,
        "std_deviation": std_deviation,
        "trial_times": trial_times
    }

if __name__ == "__main__":
    # Set the number of readings per trial (x) and the number of trials (y)
    x = 10
    y = 5   

    results = benchmark_readings(x, y)

    print("\nBenchmark Results:")
    print(f"Average Time per Trial ({x} readings): {results['average_time_per_trial']:.6f} seconds")
    print(f"Average Time per Reading: {results['average_time_per_reading']:.6f} seconds")
    print(f"Standard Deviation: {results['std_deviation']:.6f} seconds")
    print("Trial Times:", results['trial_times'])