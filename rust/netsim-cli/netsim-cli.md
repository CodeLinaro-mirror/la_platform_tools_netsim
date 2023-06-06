# Command Line Interface for Netsim (netsim)

Usage:
* `netsim [Options] <COMMAND>`

Options:
* `-v, --verbose`: Set verbose mode
* `-p, --port <PORT>`: Set custom grpc port
* `-h, --help`:    Print help information

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
* ### `help`:       Print this message or the help of the given subcommand(s)
