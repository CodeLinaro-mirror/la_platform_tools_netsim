Feature: Android UWB Ranging

  Scenario: UWB Ranging between two Android devices
    Given @adb has 2 attached devices
    And @netsim is running

    # Configure UWB parameters
    When @android:1 configures UWB channel 9 and preamble 11
    And @android:1 configures UWB session ID 12345678
    And @android:2 configures UWB channel 9 and preamble 11
    And @android:2 configures UWB session ID 12345678

    # Enable UWB
    When @android:1 enables UWB
    And @android:2 enables UWB

    # Initialize Sessions
    When @android:1 initializes UWB CONTROLLER session
    And @android:2 initializes UWB CONTROLEE session

    # Wait for addresses
    Then @android:1 UWB address is available as UWB_ADDRESS_1
    Then @android:2 UWB address is available as UWB_ADDRESS_2

    # Now valid ranging
    When @android:1 starts UWB ranging with peer {UWB_ADDRESS_2} as CONTROLLER
    And @android:2 starts UWB ranging with peer {UWB_ADDRESS_1} as CONTROLEE

    Then @android:2 UWB peer connected is connected

    # Verify Measurements
    Then @android:2 UWB distance is 0.0 meters with tolerance 0.0 meters
    And @android:2 UWB azimuth is 0.0 degrees with tolerance 5.0 degrees
    And @android:2 UWB elevation is 0.0 degrees with tolerance 5.0 degrees

  Scenario: Stop UWB Ranging between two Android devices
    Given @adb has 2 attached devices
    And @netsim is running

    # Configure UWB parameters
    When @android:1 configures UWB channel 9 and preamble 11
    And @android:1 configures UWB session ID 12345678
    And @android:2 configures UWB channel 9 and preamble 11
    And @android:2 configures UWB session ID 12345678

    # Enable UWB
    When @android:1 enables UWB
    And @android:2 enables UWB

    # Initialize Sessions
    When @android:1 initializes UWB CONTROLLER session
    And @android:2 initializes UWB CONTROLEE session

    # Wait for addresses
    Then @android:1 UWB address is available as UWB_ADDRESS_1
    Then @android:2 UWB address is available as UWB_ADDRESS_2

    # Start and stop ranging
    When @android:1 starts UWB ranging with peer {UWB_ADDRESS_2} as CONTROLLER
    And @android:2 starts UWB ranging with peer {UWB_ADDRESS_1} as CONTROLEE
    Then @android:2 UWB peer connected is connected

    When @android:1 Stops UWB Ranging
    And @android:2 Stops UWB Ranging
    Then @android:2 UWB peer disconnected is disconnected
