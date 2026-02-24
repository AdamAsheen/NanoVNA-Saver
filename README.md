# NanoVNA Saver

### What is the NanoVNA
A NanoVNA is a vector network analyser that measures the magnitude and phase of electrical signals to characterise RF components such as antennas, filters, and transmission lines across a specified frequency range. More information can be found More information can be found on the [NanoVNA official website](https://nanovna.com/).

### Our Project
The motivation behind the NanoVNA saver tool was to create a tool faster than exisiting solutions to accruately read and save data. This tool is also ableto read data from multiple NanoVNAs at the same time. While our tool is primarily to save NanoVNA readings our tool also supports a number of different NanoVNA configuration options, that being the ability to configure start and stop frequency and the ability to configure the internal NanoVNA IF bandwidth. 

### Benchmarks

### Usage
The usage of the tool is as follow, all flags are completely optional and ommision of flags will result in default values being used instead.
```bash
Usage: NanoVNA-Saver [OPTIONS]

Options:
  -s, --num-sweeps <NUM_SWEEPS>      [default: 1]
  -d, --vna-number <VNA_NUMBER>      [default: 1]
      --start-freq <START_FREQ>      [default: 50000]
      --end-freq <END_FREQ>          [default: 900000000]
  -p, --num-points <NUM_POINTS>      [default: 101]
      --num-ports <NUM_PORTS>        [default: 2]
  -i, --if-bandwidth <IF_BANDWIDTH>
  -h, --help                         Print help
```
