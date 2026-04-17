Feature: Feature Observables Verification

  Scenario: Verify mock feature counter
    Given @adb has 1 attached device
    When @avd fetch feature observables
    Then @avd observes "mock-feature" should be "42"
