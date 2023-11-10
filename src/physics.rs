use glam::{vec2, Vec2};

use crate::{chunk_iter::ChunksMutIndices, HEIGHT, WIDTH};
use rayon::prelude::*;

#[derive(Clone, Copy, Default, Debug)]
pub struct Node {
    pub pos: Vec2,
    pub last_pos: Vec2,
    pinned: bool,
}

struct Link {
    node1: usize,
    node2: usize,
    resting_distance: f32,
}

impl Link {
    fn new(nodes: &[Node], node1: usize, node2: usize) -> Self {
        Self {
            node1,
            node2,
            resting_distance: nodes[node1].pos.distance(nodes[node2].pos),
        }
    }
}

struct Flag {
    width: usize,
    height: usize,
    links: Vec<Link>,
}

impl Flag {
    fn new(nodes: &mut [Node], corner: Vec2, size: f32, width: usize, height: usize) -> Self {
        for y in 0..height {
            for x in 0..width {
                nodes[x + y * width].pos =
                    vec2(x as f32, y as f32) * (size / width as f32) + corner;
                nodes[x + y * width].last_pos = nodes[x + y * width].pos;
                nodes[x + y * width].pinned = x == 0 && (y == 0 || y == height - 1);
            }
        }

        let links = (0..height)
            .flat_map(|y| {
                let n = &nodes;
                (0..(width - 1)).map(move |x| Link::new(n, x + y * width, x + 1 + y * width))
            })
            .chain((0..width).flat_map(|x| {
                let n = &nodes;
                (0..(height - 1)).map(move |y| Link::new(n, x + y * width, x + (y + 1) * width))
            }))
            .collect();

        Self {
            width,
            height,
            links,
        }
    }
}

pub struct Physics {
    nodes: Vec<Node>,
    flags: Vec<Flag>,

    selected_nodes: Option<Vec<usize>>,
}

impl Physics {
    pub fn new() -> Physics {
        let mut nodes = [Node::default(); 200 * 130 * 2].to_vec();

        let flags = vec![
            Flag::new(&mut nodes[..200 * 130], vec2(150.0, 50.0), 500.0, 200, 130),
            Flag::new(&mut nodes[200 * 130..], vec2(150.0, 450.0), 500.0, 200, 130),
        ];
        Physics {
            nodes,
            flags,
            selected_nodes: None,
        }
    }

    fn update_pos(&mut self, gravity: Vec2, dt: f32) {
        self.nodes.iter_mut().filter(|n| !n.pinned).for_each(|n| {
            let diff = n.pos - n.last_pos;
            n.last_pos = n.pos;
            n.pos += (diff + gravity * (dt * dt)).clamp_length_max(50.0);
        });
    }

    fn apply_constraint(&mut self) {
        let factor = 0.75;
        self.nodes.iter_mut().filter(|n| !n.pinned).for_each(|n| {
            if n.pos.x > WIDTH as f32 {
                n.pos.x += factor * (WIDTH as f32 - n.pos.x);
            }
            if n.pos.x < 0.0 {
                n.pos.x -= factor * n.pos.x;
            }
            if n.pos.y > HEIGHT as f32 {
                n.pos.y += factor * (HEIGHT as f32 - n.pos.y);
            }
            if n.pos.y < 0.0 {
                n.pos.y -= factor * n.pos.y;
            }
        });
    }

    fn apply_links(&mut self) {
        let breakpoints = self
            .flags
            .iter()
            .map(|f| f.width * f.height)
            .fuse()
            .collect::<Vec<usize>>();
        let chunks: ChunksMutIndices<'_, Node> =
            ChunksMutIndices::new(&mut self.nodes, &breakpoints);

        self.flags
            .iter()
            .zip(chunks)
            .par_bridge()
            .for_each(|(flag, (nodes, size))| {
                println!("{} {}", nodes.len(), size);
                flag.links.iter().for_each(|link| {
                    let diff = nodes[link.node1].pos - nodes[link.node2].pos;
                    let dist = diff.length();
                    let force = ((link.resting_distance - dist) / dist * 0.5).min(0.001);
                    let n = diff * force;
                    if !nodes[link.node1].pinned {
                        nodes[link.node1].pos += n;
                    }
                    if !nodes[link.node2].pinned {
                        nodes[link.node2].pos -= n;
                    }
                })
            })
    }

    pub fn step(&mut self, gravity: Vec2, dt: f32) {
        self.update_pos(gravity, dt);
        self.apply_constraint();
        self.apply_links();
    }

    pub fn avoid_obstacle(&mut self, pos: Vec2, size: f32) {
        self.nodes.iter_mut().filter(|n| !n.pinned).for_each(|p| {
            let v = p.pos - pos;
            let dist2 = v.length_squared();
            let min_dist = size;
            if dist2 < min_dist * min_dist {
                let dist = dist2.sqrt();
                let n = v / dist;
                p.pos -= n * 0.1 * (dist - min_dist);
            }
        })
    }

    pub fn get_indices(&self) -> Vec<i16> {
        self.flags[0]
            .links
            .iter()
            .flat_map(|l| [l.node1 as i16, l.node2 as i16])
            .collect()
    }

    pub fn get_points(&self) -> Vec<Vec2> {
        self.nodes.iter().map(|n| n.pos).collect()
    }

    pub fn num_links(&self) -> i32 {
        self.flags.iter().map(|f| f.links.len() as i32).sum()
    }

    pub fn select_nodes(&mut self, pos: Vec2) {
        let radius = 10.0;
        let in_range = self
            .nodes
            .iter()
            .map(|n| n.pos.distance_squared(pos))
            .enumerate()
            .filter(|(_, d)| *d < radius * radius)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();
        if in_range.len() > 0 {
            self.selected_nodes = Some(in_range);
        } else {
            self.selected_nodes = None
        }
    }

    pub fn move_selected_nodes(&mut self, pos: Vec2) {
        match &self.selected_nodes {
            None => (),
            Some(nodes) => nodes.iter().for_each(|&i| {
                self.nodes[i].pos = pos;
            }),
        }
    }
}
