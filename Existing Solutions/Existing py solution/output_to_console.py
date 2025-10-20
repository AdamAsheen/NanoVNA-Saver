import pynanovna

# Initialize NanoVNA
vna = pynanovna.VNA()

# Set sweep parameters (start_freq, stop_freq, num_points)
vna.set_sweep(1.0e9, 1.4e9, 101)

# Stream raw data without calibration
stream = vna.stream()
for s11, s21, frequencies in stream:
    print(f"Frequency: {frequencies} Hz, S11: {s11}, S21: {s21}")
