use crate::*;
use std::f32::consts::*;

/// How far our eye can see:
///
/// -----------------
/// |               |
/// |               |
/// |               |
/// |@      %      %|
/// |               |
/// |               |
/// |               |
/// -----------------
///
/// If @ marks our birdie and % marks food, then a FOV_RANGE of:
///
/// - 0.1 = 10% of the map = bird sees no foods (at least in this case)
/// - 0.5 = 50% of the map = bird sees one of the foods
/// - 1.0 = 100% of the map = bird sees both foods
const FOV_RANGE: f32 = 0.25;

/// How wide our eye can see.
///
/// If @> marks our birdie (rotated to the right) and . marks the area
/// our birdie sees, then a FOV_ANGLE of:
///
/// - PI/2 = 90° =
///   -----------------
///   |             /.|
///   |           /...|
///   |         /.....|
///   |       @>......|
///   |         \.....|
///   |           \...|
///   |             \.|
///   -----------------
///
/// - PI = 180° =
///   -----------------
///   |       |.......|
///   |       |.......|
///   |       |.......|
///   |       @>......|
///   |       |.......|
///   |       |.......|
///   |       |.......|
///   -----------------
///
/// - 2 * PI = 360° =
///   -----------------
///   |...............|
///   |...............|
///   |...............|
///   |.......@>......|
///   |...............|
///   |...............|
///   |...............|
///   -----------------
///
/// Field of view depends on both FOV_RANGE and FOV_ANGLE:
///
/// - FOV_RANGE=0.4, FOV_ANGLE=PI/2:
///   -----------------
///   |       @       |
///   |     /.v.\     |
///   |   /.......\   |
///   |   ---------   |
///   |               |
///   |               |
///   |               |
///   -----------------
///
/// - FOV_RANGE=0.5, FOV_ANGLE=2*PI:
///   -----------------
///   |               |
///   |      ---      |
///   |     /...\     |
///   |    |..@..|    |
///   |     \.../     |
///   |      ---      |
///   |               |
///   -----------------
const FOV_ANGLE: f32 = PI + FRAC_PI_4;

/// How much photoreceptors there are in a single eye.
///
/// More cells means our birds will have more "crisp" vision, allowing
/// them to locate the food more precisely - but the trade-off is that
/// the evolution process will then take longer, or even fail, unable
/// to find any solution.
///
/// I've found values between 3~11 sufficient, with eyes having more
/// than ~20 photoreceptors yielding progressively worse results.
const CELLS: usize = 9;

#[derive(Debug)]
pub struct Eye {
    fov_range: f32,
    fov_angle: f32,
    cells: usize,
}

impl Eye {
    // FOV_RANGE, FOV_ANGLE & CELLS are the values we'll use during
    // simulation - but being able to create an arbitrary eye will
    // come handy during the testing:
    fn new(fov_range: f32, fov_angle: f32, cells: usize) -> Self {
        assert!(fov_range > 0.0);
        assert!(fov_angle > 0.0);
        assert!(cells > 0);

        Self { fov_range, fov_angle, cells }
    }

    pub fn cells(&self) -> usize {
        self.cells
    }

    pub fn process_vision(
        &self,
        position: na::Point2<f32>,
        rotation: na::Rotation2<f32>,
        foods: &[Food],
    ) -> Vec<f32> {
        let mut cells = vec![0.0; self.cells];

        for food in foods {
            let vec = food.position - position;

            let dist = vec.norm(); //turn it into a value 0<=x<=1

            if dist >= self.fov_range{
                continue;
            }

            let angle = na::Rotation2::rotation_between(
                &na::Vector2::y(),
                &vec,
            ).angle();

            //Since the bird is rotated account for that
            let angle = angle - rotation.angle();

            //Gets a angle value in between -PI and PI
            let angle = na::wrap(angle, -PI, PI);

            //If the angle is outside the bird's eye then skip it
            if angle < -self.fov_angle / 2.0 || angle > self.fov_angle / 2.0 {
                continue;
            }

            // Makes angle *relative* to our birdie's field of view - that is:
            // transforms it from <-FOV_ANGLE/2,+FOV_ANGLE/2> to <0,FOV_ANGLE>.
            //
            // After this operation:
            // - an angle of 0° means "the beginning of the FOV",
            // - an angle of self.fov_angle means "the ending of the FOV".
            let angle = angle + self.fov_angle / 2.0;

            // Since this angle is now in range <0,FOV_ANGLE>, by dividing it by
            // FOV_ANGLE, we transform it to range <0,1>.
            //
            // The value we get can be treated as a percentage, that is:
            //
            // - 0.2 = the food is seen by the "20%-th" eye cell
            //         (practically: it's a bit to the left)
            //
            // - 0.5 = the food is seen by the "50%-th" eye cell
            //         (practically: it's in front of our birdie)
            //
            // - 0.8 = the food is seen by the "80%-th" eye cell
            //         (practically: it's a bit to the right)
            let cell = angle / self.fov_angle;

            // With cell in range <0,1>, by multiplying it by the number of
            // cells we get range <0,CELLS> - this corresponds to the actual
            // cell index inside our `cells` array.
            //
            // Say, we've got 8 eye cells:
            // - 0.2 * 8 = 20% * 8 = 1.6 ~= 1 = second cell (indexing from 0!)
            // - 0.5 * 8 = 50% * 8 = 4.0 ~= 4 = fifth cell
            // - 0.8 * 8 = 80% * 8 = 6.4 ~= 6 = seventh cell
            let cell = cell * (self.cells as f32);

            // Our `cell` is of type `f32` - before we're able to use it to
            // index an array, we have to convert it to `usize`.
            //
            // We're also doing `.min()` to cover an extreme edge case: for
            // cell=1.0 (which corresponds to a food being maximally to the
            // right side of our birdie), we'd get `cell` of `cells.len()`,
            // which is one element *beyond* what the `cells` array contains
            // (its range is <0, cells.len()-1>).
            //
            // Being honest, I've only caught this thanks to unit tests we'll
            // write in a moment, so if you consider my explanation
            // insufficient (pretty fair!), please feel free to drop the
            // `.min()` part later and see which tests fail - and why!
            let cell = (cell as usize).min(cells.len() - 1);

            // Energy is inversely proportional to the distance between our
            // birdie and the currently checked food; that is - an energy of:
            //
            // - 0.0001 = food is barely in the field of view (i.e. far away),
            // - 1.0000 = food is right in front of the bird.
            //
            // We could also model energy in reverse manner - "the higher the
            // energy, the further away the food" - but from what I've seen, it
            // makes the learning process a bit harder.
            //
            // As always, feel free to experiment! -- overall this isn't the
            // only way of implementing eyes.
            let energy = (self.fov_range - dist) / self.fov_range;

            cells[cell] += energy;
        }
    
        cells
    }
}

impl Default for Eye {
    fn default() -> Self {
        Self::new(FOV_RANGE, FOV_ANGLE, CELLS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    struct TestCase {
        foods: Vec<Food>,
        fov_range: f32,
        fov_angle: f32,
        x: f32,
        y: f32,
        rot: f32,
        expected_vision: &'static str,
    }

    const TEST_EYE_CELLS: usize = 13;

    impl TestCase {
        fn run(self) {
            let eye = Eye::new(self.fov_range, self.fov_angle, TEST_EYE_CELLS);

            let actual_vision = eye.process_vision(
                na::Point2::new(self.x, self.y),
                na::Rotation2::new(self.rot),
                &self.foods,
            );

            let actual_vision: Vec<_> = actual_vision
            .into_iter()
            .map(|cell| {
                // As a reminder, the higher cell's value, the closer
                // the food is:

                if cell >= 0.7 {
                    // <0.7, 1.0>
                    // food is right in front of us
                    "#"
                } else if cell >= 0.3 {
                    // <0.3, 0.7)
                    // food is somewhat further
                    "+"
                } else if cell > 0.0 {
                    // <0.0, 0.3)
                    // food is pretty far away
                    "."
                } else {
                    // 0.0
                    // no food in sight, this cell sees empty space
                    " "
                }
            })
            .collect();
    
            let actual_vision = actual_vision.join("");

            assert_eq!(actual_vision, self.expected_vision);
        }
    }

    fn food(x: f32, y: f32) -> Food {
        Food {
            position: na::Point2::new(x, y),
        }
    }
    
    /// During tests in this module, we're using a world that looks
    /// like this:
    ///
    /// -------------
    /// |           |
    /// |           |
    /// |     @     |
    /// |     v     | `v` here indicates where the birdie is looking at
    /// |           |
    /// |     %     |
    /// -------------
    ///
    /// Each test gradually reduces our birdie's field of view and
    /// checks what the birdie sees:
    ///
    /// -------------
    /// |           |
    /// |           |
    /// |     @     |
    /// |    /v\    |
    /// |  /.....\  | `.` here indicates the part of view the birdie sees
    /// |/....%....\|
    /// -------------
    ///
    /// -------------
    /// |           |
    /// |           |
    /// |     @     |
    /// |    /v\    |
    /// |  /.....\  |
    /// |     %     |
    /// -------------
    ///
    /// -------------
    /// |           |
    /// |           |
    /// |     @     |
    /// |    /.\    |
    /// |           |
    /// |     %     |
    /// -------------
    ///
    /// Over time, what we see is the food gradually disappearing
    /// into the emptiness:
    ///
    /// (well, technically the food and bird remain stationary - it's
    /// only the birdie's own field of view that gets reduced.)

    #[test_case(1.0, "      +      ")] // Food is inside the FOV
    #[test_case(0.9, "      +      ")] // ditto
    #[test_case(0.8, "      +      ")] // ditto
    #[test_case(0.7, "      .      ")] // Food slowly disappears
    #[test_case(0.6, "      .      ")] // ditto
    #[test_case(0.5, "             ")] // Food disappeared!
    #[test_case(0.4, "             ")]
    #[test_case(0.3, "             ")]
    #[test_case(0.2, "             ")]
    #[test_case(0.1, "             ")]
    fn fov_ranges(fov_range: f32, expected_vision: &'static str) {
        TestCase {
            foods: vec![food(0.5, 1.0)],
            fov_angle: FRAC_PI_2,
            x: 0.5,
            y: 0.5,
            rot: 0.0,
            fov_range,
            expected_vision,
        }.run()
    }

    /// World:
    ///
    /// -------------
    /// |           |
    /// |           |
    /// |%    @     |
    /// |     v     |
    /// |           |
    /// -------------
    ///
    /// Test cases:
    ///
    /// -------------
    /// |...........|
    /// |...........|
    /// |%....@.....|
    /// |.....v.....|
    /// |...........|
    /// -------------
    ///
    /// -------------
    /// |...........|
    /// |...........|
    /// |%...<@.....|
    /// |...........|
    /// |...........|
    /// -------------
    ///
    /// -------------
    /// |...........|
    /// |.....^.....|
    /// |%....@.....|
    /// |...........|
    /// |...........|
    /// -------------
    ///
    /// -------------
    /// |...........|
    /// |...........|
    /// |%....@>....|
    /// |...........|
    /// |...........|
    /// -------------
    ///
    /// ... and so on, until we do a full circle, 360° rotation:
    #[test_case(0.00 * PI, "         +   ")] // Food is to our right
    #[test_case(0.25 * PI, "        +    ")]
    #[test_case(0.50 * PI, "      +      ")] // Food is in front of us
    #[test_case(0.75 * PI, "    +        ")]
    #[test_case(1.00 * PI, "   +         ")] // Food is to our left
    #[test_case(1.25 * PI, " +           ")]
    #[test_case(1.50 * PI, "            +")] // Food is behind us
    #[test_case(1.75 * PI, "           + ")] // (we continue to see it
    #[test_case(2.00 * PI, "         +   ")] // due to 360° fov_angle.)
    #[test_case(2.25 * PI, "        +    ")]
    #[test_case(2.50 * PI, "      +      ")]
    fn rotations(rot: f32, expected_vision: &'static str) {
        TestCase {
            foods: vec![food(0.0, 0.5)],
            fov_range: 1.0,
            fov_angle: 2.0 * PI,
            x: 0.5,
            y: 0.5,
            rot,
            expected_vision,
        }.run()
    }

    /// World:
    ///
    /// ------------
    /// |          |
    /// |         %|
    /// |          |
    /// |         %|
    /// |          |
    /// ------------
    ///
    /// Test cases for the X axis:
    ///
    /// ------------
    /// |          |
    /// |        /%|
    /// |       @>.|
    /// |        \%|
    /// |          |
    /// ------------
    ///
    /// ------------
    /// |        /.|
    /// |      /..%|
    /// |     @>...|
    /// |      \..%|
    /// |        \.|
    /// ------------
    ///
    /// ... and so on, going further left
    ///     (or, from the bird's point of view - going _back_)
    ///
    /// Test cases for the Y axis:
    ///
    /// ------------
    /// |     @>...|
    /// |       \.%|
    /// |        \.|
    /// |         %|
    /// |          |
    /// ------------
    ///
    /// ------------
    /// |      /...|
    /// |     @>..%|
    /// |      \...|
    /// |        \%|
    /// |          |
    /// ------------
    ///
    /// ... and so on, going further down
    ///     (or, from the bird's point of view - going _right_)

    // Checking the X axis:
    // (you can see the bird is "flying away" from the foods)
    #[test_case(0.9, 0.5, "#           #")]
    #[test_case(0.8, 0.5, "  #       #  ")]
    #[test_case(0.7, 0.5, "   +     +   ")]
    #[test_case(0.6, 0.5, "    +   +    ")]
    #[test_case(0.5, 0.5, "    +   +    ")]
    #[test_case(0.4, 0.5, "     + +     ")]
    #[test_case(0.3, 0.5, "     . .     ")]
    #[test_case(0.2, 0.5, "     . .     ")]
    #[test_case(0.1, 0.5, "     . .     ")]
    #[test_case(0.0, 0.5, "             ")]
    //
    // Checking the Y axis:
    // (you can see the bird is "flying alongside" the foods)
    #[test_case(0.5, 0.0, "            +")]
    #[test_case(0.5, 0.1, "          + .")]
    #[test_case(0.5, 0.2, "         +  +")]
    #[test_case(0.5, 0.3, "        + +  ")]
    #[test_case(0.5, 0.4, "      +  +   ")]
    #[test_case(0.5, 0.6, "   +  +      ")]
    #[test_case(0.5, 0.7, "  + +        ")]
    #[test_case(0.5, 0.8, "+  +         ")]
    #[test_case(0.5, 0.9, ". +          ")]
    #[test_case(0.5, 1.0, "+            ")]
    fn positions(x: f32, y: f32, expected_vision: &'static str) {
        TestCase {
            foods: vec![food(1.0, 0.4), food(1.0, 0.6)],
            fov_range: 1.0,
            fov_angle: FRAC_PI_2,
            rot: 3.0 * FRAC_PI_2,
            x,
            y,
            expected_vision,
        }.run()
    }

    /// World:
    ///
    /// ------------
    /// |%        %|
    /// |          |
    /// |%        %|
    /// |    @>    |
    /// |%        %|
    /// |          |
    /// |%        %|
    /// ------------
    ///
    /// Test cases:
    ///
    /// ------------
    /// |%        %|
    /// |         /|
    /// |%      /.%|
    /// |    @>....|
    /// |%      \.%|
    /// |         \|
    /// |%        %|
    /// ------------
    ///
    /// ------------
    /// |%      /.%|
    /// |      /...|
    /// |%    /...%|
    /// |    @>....|
    /// |%    \...%|
    /// |      \...|
    /// |%      \.%|
    /// ------------
    ///
    /// ------------
    /// |%........%|
    /// |\.........|
    /// |% \......%|
    /// |    @>....|
    /// |% /......%|
    /// |/.........|
    /// |%........%|
    /// ------------
    ///
    /// ... and so on, until we reach the full, 360° FOV
    #[test_case(0.25 * PI, " +         + ")] // FOV is narrow = 2 foods
    #[test_case(0.50 * PI, ".  +     +  .")]
    #[test_case(0.75 * PI, "  . +   + .  ")] // FOV gets progressively
    #[test_case(1.00 * PI, "   . + + .   ")] // wider and wider...
    #[test_case(1.25 * PI, "   . + + .   ")]
    #[test_case(1.50 * PI, ".   .+ +.   .")]
    #[test_case(1.75 * PI, ".   .+ +.   .")]
    #[test_case(2.00 * PI, "+.  .+ +.  .+")] // FOV is the widest = 8 foods
    fn fov_angles(fov_angle: f32, expected_vision: &'static str) {
        TestCase {
            foods: vec![
                food(0.0, 0.0),
                food(0.0, 0.33),
                food(0.0, 0.66),
                food(0.0, 1.0),
                food(1.0, 0.0),
                food(1.0, 0.33),
                food(1.0, 0.66),
                food(1.0, 1.0),
            ],
            fov_range: 1.0,
            x: 0.5,
            y: 0.5,
            rot: 3.0 * FRAC_PI_2,
            fov_angle,
            expected_vision,
        }.run()
    }
}