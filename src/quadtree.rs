use std::fmt::Debug;
use gpx::GPXPoint;
use geo::*;
use chrono;

const CAPACITY: usize = 5;

#[derive(Debug)]
pub struct QuadTree<T> {
    depth: u32,
    elements: Vec<T>,
    children: Option<[Box<QuadTree<T>>; 4]>,
    x: f64,
    y: f64,
    size: f64,
}

pub trait Geospatial {
    fn x(&self) -> f64;
    fn y(&self) -> f64;
    fn date(&self) -> chrono::DateTime<chrono::FixedOffset>;
}

impl Geospatial for GPXPoint {
    fn x(&self) -> f64 {
        self.lon
    }
    fn y(&self) -> f64 {
        self.lat
    }
    fn date(&self) -> chrono::DateTime<chrono::FixedOffset> {
        self.time.unwrap()
    }
}

// TODO loosen the Copy type constraint
impl<T> QuadTree<T> where T: Debug + Geospatial + Copy {
    pub fn new(x: f64, y: f64, size: f64, depth: u32) -> QuadTree<T> {
        QuadTree {
            elements: Vec::new(),
            depth: depth,
            children: None,
            x: x,
            y: y,
            size: size,
        }
    }

    pub fn root() -> QuadTree<T> {
        QuadTree {
            elements: Vec::new(),
            depth: 0,
            children: None,
            x: 0.0,
            y: 0.0,
            size: 1000.0,
        }
    }

    fn split(&mut self) {
        let hs = (self.size / 2.0);
        let qs = (self.size / 4.0);
        self.children = Some([
            Box::new(QuadTree::new(self.x - qs, self.y - qs, hs, self.depth + 1)),
            Box::new(QuadTree::new(self.x - qs, self.y + qs, hs, self.depth + 1)),
            Box::new(QuadTree::new(self.x + qs, self.y - qs, hs, self.depth + 1)),
            Box::new(QuadTree::new(self.x + qs, self.y + qs, hs, self.depth + 1)),
        ]);
    }

    fn contains(&self, item: &T) -> bool {
        let hs = (self.size / 2.0);
        if (item.x() < self.x + hs)  &&
           (item.y() < self.y + hs)  &&
           (item.x() >= self.x - hs ) &&
           (item.y() >= self.y - hs ) {
               return true
        }
        return false
    }

    pub fn get(&self, x: f64, y: f64, rad: f64) -> Vec<T> {
        let mut accum = Vec::new();
        let hs = (self.size / 2.0);
        if ((x + rad > self.x - hs) || (y + rad > self.y - hs) ||
            (y - rad < self.y + hs) || (x - rad < self.x + hs)) {
            for elem in self.elements.iter() {
                if (elem.x() > x - rad && elem.x() < x + rad &&
                    elem.y() > y - rad && elem.y() < y + rad)
                {
                    accum.push(elem.clone())
                }
            }
            match self.children {
                None => { /* a leaf node */ },
                Some(ref children) => {
                    for child in children {
                        accum.extend(child.get(x, y, rad))
                    }
                }
            }
        }
        return accum;
    }

    pub fn insert(&mut self, item: T) {
        if self.contains(&item) {
            if self.elements.len() < CAPACITY {
                self.elements.push(item);
            } else {
                if self.children.is_none() {
                    self.split();
                }
                match self.children {
                    None => panic!("Should have split before now"),
                    Some(ref mut children) => {
                       for child in children {
                           child.insert(item);
                       }
                   }
                }
            }
        }
    }
}
