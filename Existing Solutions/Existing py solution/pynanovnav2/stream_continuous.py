"""
Real-time streaming example using pynanovnav2.
This demonstrates continuous data acquisition from NanoVNA V2.
"""

import sys
sys.path.append("d:\\Git repo\\pynanovnav2\\src")

from pynanovnav2.nanovnav2 import NanoVNAV2
import time

def stream_data(port="COM9S", start_freq=50e6, stop_freq=900e6, step_freq=10e6, num_sweeps=10):
    """
    Stream data from NanoVNA V2 for multiple sweeps.
    
    Args:
        port: Serial port for NanoVNA
        start_freq: Start frequency in Hz
        stop_freq: Stop frequency in Hz
        step_freq: Step frequency in Hz
        num_sweeps: Number of sweeps to perform
    """
    
    print(f"Initializing NanoVNA V2 streaming on {port}...")
    print(f"Frequency range: {start_freq/1e6:.1f} MHz to {stop_freq/1e6:.1f} MHz")
    print(f"Step size: {step_freq/1e6:.1f} MHz")
    print(f"Number of sweeps: {num_sweeps}")
    print()
    
    try:
        with NanoVNAV2(port, debug=False, useNumpy=True) as vna:
            # Get device info
            info = vna._get_id()
            print(f"Device: {info['title']}, Firmware: {info['firmware']}")
            
            # Configure sweep
            vna._set_sweep_range(start_freq, stop_freq, step_freq)
            
            # Perform multiple sweeps
            for sweep_num in range(num_sweeps):
                print(f"\n--- Sweep {sweep_num + 1}/{num_sweeps} ---")
                start_time = time.time()
                
                # Query data
                data = vna._query_trace()
                
                elapsed = time.time() - start_time
                
                # Display summary
                num_points = len(data['freq'])
                print(f"Points acquired: {num_points}")
                print(f"Time elapsed: {elapsed:.3f} seconds")
                print(f"Rate: {num_points/elapsed:.1f} points/second")
                
                # Show first and last frequency
                if num_points > 0:
                    print(f"First frequency: {data['freq'][0]/1e6:.3f} MHz")
                    print(f"Last frequency: {data['freq'][-1]/1e6:.3f} MHz")
                
                # Optional: Add delay between sweeps
                if sweep_num < num_sweeps - 1:
                    time.sleep(0.5)
            
            print("\nStreaming complete!")
            
    except KeyboardInterrupt:
        print("\n\nStreaming interrupted by user.")
    except Exception as e:
        print(f"\nError occurred: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    # Change these parameters as needed
    PORT = "COM3"  # Update with your NanoVNA port
    START_FREQ = 50e6  # 50 MHz
    STOP_FREQ = 900e6  # 900 MHz
    STEP_FREQ = 5e6    # 5 MHz
    NUM_SWEEPS = 10
    
    stream_data(PORT, START_FREQ, STOP_FREQ, STEP_FREQ, NUM_SWEEPS)
