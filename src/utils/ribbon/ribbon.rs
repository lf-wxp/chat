use derivative::Derivative;

use super::{
  point::Point,
  section::Section,
  util::{random, Dir, Position},
};

#[derive(Derivative)]
#[derivative(Debug, Default, PartialEq, Clone)]
pub struct Ribbon {
  #[derivative(Default(value = "100.0"))]
  hide: f64,
  #[derivative(Default(value = "200.0"))]
  min: f64,
  max: f64,
  dir: Dir,
  start_x: f64,
  start_y: f64,
  pub sections: Vec<Section>,
  width: f64,
  height: f64,
  move_speed: f64,
  color_speed: f64,
  pub is_done: bool,
}

impl Ribbon {
  pub fn new(
    width: f64,
    height: f64,
    position: Position,
    hide: f64,
    color_speed: f64,
    move_speed: f64,
  ) -> Ribbon {
    let dir = if random(1..10) > 5 {
      Dir::Right
    } else {
      Dir::Left
    };
    let min = 0.0 - hide;
    let max = width + hide;

    let start_x = if dir == Dir::Right { min } else { max };
    let mut start_y = (random(0..(height as u16)) as f64).round();

    match position {
      Position::Top => {
        start_y = hide;
      }
      Position::Middle => {
        start_y = height / 2.0;
      }
      Position::Bottom => {
        start_y = height - hide;
      }
      _ => {}
    }
    let mut ribbon = Ribbon {
      dir,
      min,
      max,
      height,
      width,
      start_x,
      start_y,
      move_speed,
      color_speed,
      is_done: false,
      sections: vec![],
      ..Ribbon::default()
    };

    ribbon.create_sections();
    ribbon
  }

  fn create_sections(&mut self) -> () {
    let mut color = f64::from(random(0..360));
    let mut delay = 0.0;
    let mut point_1 = Point::new(self.start_x, self.start_y);
    let mut point_2 = Point::new(self.start_x, self.start_y);
    let mut move_x = 0.0;
    let mut move_y = 0.0;
    let mut stop = 500;
    loop {
      if stop <= 0 {
        break;
      };
      stop -= 1;
      let rand_num = || f64::from(random(0..2));
      move_x = ((rand_num() - 0.2) * self.move_speed).round();
      move_y = ((rand_num() - 0.5) * self.height * 0.25).round();
      let mut point_3 = Point::from(point_2.clone());

      match self.dir {
        Dir::Right => {
          point_3.add(move_x, move_y);
          if &point_2.x >= &self.max {
            break;
          }
        }
        Dir::Left => {
          point_3.subtract(move_x, move_y);
          if &point_2.x <= &self.min {
            break;
          }
        }
      }
      self.sections.push(Section {
        points: [
          Point::from(point_1),
          Point::from(point_2),
          Point::from(point_3),
        ],
        color,
        delay,
        dir: self.dir,
        alpha: 0.0,
        phase: 0.0,
      });

      point_1.copy(point_2);
      point_2.copy(point_3);
      delay += 4.0;
      color += self.color_speed;
    }
  }

  pub fn set_done(&mut self) -> () {
    self.is_done = true;
  }
}
