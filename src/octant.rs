#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Polar {
    pub vel: f64, //0 .. 1
    pub dir: f64, //0 .. 1 angle, 0 is top/up
}

pub fn xy_to_vel_cir(x: f64, y: f64) -> Polar {
    if x == 0.0 && y == 0.0 { return Polar {vel: 0.0, dir: 0.0} }
    let vel = f64::sqrt((x*x) + (y*y));
    let vel = vel.clamp(0.0,1.0);
    let dir_rad = f64::atan2(x, y);
    let dir_cir = dir_rad / (std::f64::consts::PI * 2.0); // range is -0.5 .. 0.5 and 0 is at the top
    let dir_cir = if dir_cir < 0.0 { dir_cir + 1.0 } else { dir_cir }; //range is 0 .. 1 and 0 is at the top
    let dir_cir = dir_cir.clamp(0.0,1.0);

    Polar { vel, dir: dir_cir }
}

#[derive(Debug,Copy,Clone,PartialEq,Eq)]
pub enum OctantSection {
    Octant(u8),
    Center,
}

const SPACE_BETWEEN_OCTANTS_DEG:f64 = 5.0;
const PADD:f64 = (SPACE_BETWEEN_OCTANTS_DEG / 360.0) / 2.0;

fn within_padd(n: f64, from: f64, to: f64) -> bool {
    (from + PADD) <= n && n <= (to - PADD)
}

pub fn polar_to_octant(p: Polar) -> Option<OctantSection> {
    let p = Polar { vel: p.vel, dir: (p.dir + (1.0/16.0)) % 1.0};
    if p.vel < 0.6 {
        Some(OctantSection::Center)
    } else if within_padd(p.dir, 0.0, 1.0/8.0) {
        Some(OctantSection::Octant(0))
    } else if within_padd(p.dir, 1.0/8.0, 2.0/8.0) {
        Some(OctantSection::Octant(1))
    } else if within_padd(p.dir, 2.0/8.0, 3.0/8.0) {
        Some(OctantSection::Octant(2))
    } else if within_padd(p.dir, 3.0/8.0, 4.0/8.0) {
        Some(OctantSection::Octant(3))
    } else if within_padd(p.dir, 4.0/8.0, 5.0/8.0) {
        Some(OctantSection::Octant(4))
    } else if within_padd(p.dir, 5.0/8.0, 6.0/8.0) {
        Some(OctantSection::Octant(5))
    } else if within_padd(p.dir, 6.0/8.0, 7.0/8.0) {
        Some(OctantSection::Octant(6))
    } else if within_padd(p.dir, 7.0/8.0, 1.0) {
        Some(OctantSection::Octant(7))
    } else {
        None
    }
}
