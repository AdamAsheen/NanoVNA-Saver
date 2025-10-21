import time
import statistics
import pynanovna
import logging


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

    # Get access to serial port for byte tracking
    serial_port = None
    if hasattr(vna, 'dev') and hasattr(vna.dev, 'serial'):
        serial_port = vna.dev.serial
    elif hasattr(vna, 'serial'):
        serial_port = vna.serial
    elif hasattr(vna, '_serial'):
        serial_port = vna._serial

    trial_times = []
    trial_bytes = []

    for trial in range(y):
        # Track bytes received
        bytes_received_start = serial_port.in_waiting if serial_port else 0
        
        start_time = time.time()

        # Capture x readings
        stream = vna.stream()
        bytes_this_trial = 0
        for _ in range(x):
            s11, s21, frequencies = next(stream)
            
            # Estimate bytes received based on data size
            # S11 and S21 are complex arrays, frequencies is array
            # Each complex number is 2 floats (8 bytes each) = 16 bytes
            # Each frequency is 1 float = 8 bytes
            num_points = len(frequencies) if hasattr(frequencies, '__len__') else 101
            bytes_per_reading = (num_points * 16 * 2) + (num_points * 8)  # S11 + S21 + freq
            bytes_this_trial += bytes_per_reading
            
            print(f"Frequency: {frequencies} Hz, S11: {s11}")

        end_time = time.time()
        trial_time = end_time - start_time
        trial_times.append(trial_time)
        trial_bytes.append(bytes_this_trial)

    # Calculate statistics
    average_time_per_trial = sum(trial_times) / len(trial_times)
    average_time_per_reading = average_time_per_trial / x
    std_deviation = statistics.stdev(trial_times) if len(trial_times) > 1 else 0
    
    # Calculate byte statistics
    average_bytes_per_trial = sum(trial_bytes) / len(trial_bytes)
    average_bytes_per_reading = average_bytes_per_trial / x
    throughput_kbps = (average_bytes_per_trial / average_time_per_trial) / 1024  # KB/s

    return {
        "average_time_per_trial": average_time_per_trial,
        "average_time_per_reading": average_time_per_reading,
        "std_deviation": std_deviation,
        "trial_times": trial_times,
        "average_bytes_per_trial": average_bytes_per_trial,
        "average_bytes_per_reading": average_bytes_per_reading,
        "throughput_kbps": throughput_kbps,
        "trial_bytes": trial_bytes
    }

if __name__ == "__main__":
    # Set the number of readings per trial (x) and the number of trials (y)
    x = 10
    y = 1

    results = benchmark_readings(x, y)

    print("\nBenchmark Results:")
    print(f"Average Time per Trial ({x} readings): {results['average_time_per_trial']:.6f} seconds")
    print(f"Average Time per Reading: {results['average_time_per_reading']:.6f} seconds")
    print(f"Standard Deviation: {results['std_deviation']:.6f} seconds")
    print(f"\nData Transfer:")
    print(f"Average Bytes per Trial: {results['average_bytes_per_trial']:,.0f} bytes ({results['average_bytes_per_trial']/1024:.2f} KB)")
    print(f"Average Bytes per Reading: {results['average_bytes_per_reading']:,.0f} bytes")
    print(f"Throughput: {results['throughput_kbps']:.2f} KB/s")
    print(f"\nTrial Times: {results['trial_times']}")
    print(f"Trial Bytes: {results['trial_bytes']}")