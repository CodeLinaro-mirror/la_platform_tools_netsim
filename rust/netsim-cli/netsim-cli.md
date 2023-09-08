# Command Line Interface for Netsim (netsim)

Usage:
* `netsim [Options] <COMMAND>`

Options:
* `-v, --verbose`: Set verbose mode
* `-p, --port <PORT>`: Set custom grpc port
* `    --vsock <VSOCK>`: Set vsock cid:port pair
* `-h, --help`: Print help information

## Commands:
* ### `version`:    Print Netsim version information
    * Usage: `netsim version`
* ### `radio`:      Control the radio state of a device
    * Usage: `netsim radio <RADIO_TYPE> <STATUS> <NAME>`
    * Arguments:
        * \<RADIO_TYPE\>:   Radio type [possible values: ble, classic, wifi, uwb]
        * \<STATUS\>:       Radio status [possible values: up, down]
        * \<NAME\>:         Device name
* ### `move`:       Set the device location
    * Usage: `netsim move <NAME> <X> <Y> [Z]`
    * Arguments:
        * \<NAME\>:         Device name
        * \<X\>:            x position of device
        * \<Y\>:            y position of device
        * [Z]:              Optional z position of device
* ### `devices`:    Display device(s) information
    * Usage: `netsim devices [OPTIONS]`
    * Options:
        * `-c, --continuous`:    Continuously print device(s) information every second
* ### `beacon`: A chip that sends advertisements at a set interval
    * Usage: `netsim beacon <COMMAND>`
    * #### Commands:
        * `create`: Create a beacon chip
            * Usage: `netsim beacon create <COMMAND>`
                * ##### Commands:
                    * `ble`: Create a Bluetooth low-energy beacon chip
                        * Usage: `netsim beacon create ble [DEVICE_NAME | DEVICE_NAME CHIP_NAME] [OPTIONS]`
                        * Arguments:
                            * \[DEVICE_NAME\]: Optional name of the device to create. A default name will be generated if not supplied
                            * \[CHIP_NAME\]: Optional name of the beacon chip to create within the new device. May only be specified if DEVICE_NAME is specified. A default name will be generated if not supplied
                        * Options:
                            * `--interval`: Set the advertise interval in ms
        * `patch`: Modify a beacon chip
            * Usage: `netsim beacon patch <COMMAND>`
                * ##### Commands:
                    * `ble`: Modify a Bluetooth low-energy beacon chip
                        * Usage: `netsim beacon patch ble <DEVICE_NAME> <CHIP_NAME> <OPTIONS>`
                        * Arguments:
                            * \<DEVICE_NAME\>: Name of the device that contains the beacon chip
                            * \<CHIP_NAME\>: Name of the beacon chip to modify
                        * Options:
                            * `--interval`: Set the advertise interval in ms
        * `remove`: Remove a beacon chip
            * Usage: `netsim beacon remove <DEVICE_NAME> [CHIP_NAME]`
            * Arguments:
                * \<DEVICE_NAME\>: Name of the device to remove
                * \[CHIP_NAME\]: Optional name of the beacon chip to remove
* ### `reset`:      Reset Netsim device scene
    * Usage: `netsim reset`
* ### `capture`:       Control the packet capture functionalities with commands: list, patch, get [aliases: pcap]
    * Usage: `netsim capture <COMMAND>`
    * #### Commands
        * `list`:   List currently available Captures (packet captures)
            * Usage: `netsim capture list [PATTERNS]...`
            * Arguments:
                * [PATTERNS]...:    Optional strings of pattern for captures to list. Possible filter fields
                                    include ID, Device Name, and Chip Kind
            * Options:
                * `-c, --continuous`:    Continuously print Capture information every second
        * `patch`:  Patch a Capture source to turn packet capture on/off
            * Usage: `netsim capture patch <STATE> [PATTERNS]...`
            * Arguments:
                * \<STATE\>:        Packet capture state [possible values: on, off]
                * [PATTERNS]...:  Optional strings of pattern for captures to patch. Possible filter fields
                                    include ID, Device Name, and Chip Kind
        * `get`:    Download the packet capture content
            * Usage: `netsim capture get [OPTIONS] [PATTERNS]...`
            * Arguments:
                * [PATTERNS]...:    Optional strings of pattern for captures to get. Possible filter fields
                                    include ID, Device Name, and Chip Kind
            * Options:
                * `-o, --location`: Directory to store downloaded capture file(s)
* ### `gui`:        Opens netsim Web UI
* ### `artifact`:   Opens netsim artifacts directory (log, pcaps)
* ### `help`:       Print this message or the help of the given subcommand(s)
