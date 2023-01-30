# Command Line Interface for Netsim (netsim-cli)

USAGE:
* `netsim-cli <SUBCOMMAND>`

OPTIONS:
* -`h, --help`:    Print help information

## SUBCOMMANDS:
* ### `capture`:           Control the packet capture for one or all devices
    * USAGE:
        * `netsim-cli capture <STATE> <SERIAL>`

    * ARGS:
        * \<STATE\>:     Capture state (true/false)
        * \<SERIAL\>:    Device serial
* ### `devices`:           Display device(s) information
    * USAGE:
        * `netsim-cli devices`
* ### `help`:              Print this message or the help of the given subcommand(s)
* ### `move`:              Set the device location
    * USAGE:
        * `netsim-cli move <SERIAL> <X> <Y> <Z>`

    * ARGS:
        * \<SERIAL\>:    Device serial
        * \<X\>:         x position of device
        * \<Y\>:         y position of device
        * \<Z\>:         z position of device
* ### `radio`:             Control the radio state of a device
    * USAGE:
        * `netsim-cli radio <BT_TYPE> <STATUS> <SERIAL>`

    * ARGS:
        * \<BT_TYPE\>    Radio type
        * \<STATUS\>     Radio status (up/down)
        * \<SERIAL\>     Device serial*`
* ### `reset`:             Reset Netsim device scene
    * USAGE:
        * `netsim-cli reset`
* ### `set-visibility`:    Toggle visibility of a device
    * USAGE:
        * `netsim-cli set-visibility <SERIAL> <STATE>`

    * ARGS:
        * \<SERIAL\>    Device serial
        * \<STATE\>     Visibility state (on/off)
* ### `version`:          Print Netsim version information
    * USAGE:
        * `netsim-cli version`
