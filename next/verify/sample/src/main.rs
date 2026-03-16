// use features::Features; // Already used below

pub struct CalculatorWorld {
    display: i32,
}

impl CalculatorWorld {
    /// STEP: Given the calculator is clear
    async fn clear(&mut self) {
        self.display = 0;
        println!("Display cleared in CalculatorWorld");
    }

    /// STEP: When I add (\d+)
    async fn add(&mut self, val: i32) {
        self.display += val;
        println!("Added {} to CalculatorWorld", val);
    }

    /// STEP: Then the display should be (\d+)
    async fn check_display(&mut self, expected: i32) {
        assert_eq!(self.display, expected);
        println!("Checked display {} == {}", self.display, expected);
    }
}

use features::Features;

pub fn setup_world() -> (Features<CalculatorWorld>, CalculatorWorld) {
    let mut features = Features::<CalculatorWorld>::new();

    // Include the generated glue code
    mod glue {
        use super::*;
        include!(env!("GLUE_RS"));
    }

    glue::register_steps(&mut features);

    let world = CalculatorWorld { display: 0 };
    (features, world)
}

#[tokio::main]
async fn main() {
    let (features, mut world) = setup_world();

    let feature = include_str!("../features/calculator.feat");

    features.execute_from_memory(feature, &mut world).await;
}
