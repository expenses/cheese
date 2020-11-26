use cgmath::Point2;
use ordered_float::OrderedFloat;
use spade::{
    delaunay::{
        CdtEdge, ConstrainedDelaunayTriangulation, FaceHandle, PositionInTriangulation,
        VertexHandle,
    },
    kernels::FloatKernel,
};
use std::hash::{Hash, Hasher};
use ultraviolet::Vec2;

pub struct MapHandle {
    top_left: Point2<f32>,
    top_right: Point2<f32>,
    bottom_left: Point2<f32>,
    bottom_right: Point2<f32>,
}

pub struct Map {
    dlt: ConstrainedDelaunayTriangulation<Point2<f32>, FloatKernel>,
    pub updated_this_tick: bool,
}

impl Map {
    pub fn new() -> Self {
        let mut this = Self {
            dlt: ConstrainedDelaunayTriangulation::with_tree_locate(),
            updated_this_tick: false,
        };

        this.insert(Vec2::new(0.0, 0.0), Vec2::new(200.0, 200.0));
        this
    }

    pub fn edges(&self) -> impl Iterator<Item = (Vec2, Vec2, bool)> + '_ {
        self.dlt.edges().map(move |edge| {
            let from = point_to_vec2(*edge.from());
            let to = point_to_vec2(*edge.to());
            let is_constraint = self.dlt.is_constraint_edge(edge.fix());
            (from, to, is_constraint)
        })
    }

    fn locate(&self, point: Vec2) -> Option<TriangleRef> {
        match self.dlt.locate(&Point2::new(point.x, point.y)) {
            PositionInTriangulation::InTriangle(triangle) => {
                Some(TriangleRef::new(triangle, point))
            }
            // These two seem very unlikely.
            PositionInTriangulation::OnPoint(_) => None,
            PositionInTriangulation::OnEdge(_) => None,
            PositionInTriangulation::OutsideConvexHull(_) => None,
            PositionInTriangulation::NoTriangulationPresent => None,
        }
    }

    pub fn insert(&mut self, center: Vec2, dimensions: Vec2) -> Option<MapHandle> {
        let tl = center - dimensions / 2.0;
        let br = center + dimensions / 2.0;

        let top_left = Point2::new(tl.x, tl.y);
        let top_right = Point2::new(br.x, tl.y);
        let bottom_left = Point2::new(tl.x, br.y);
        let bottom_right = Point2::new(br.x, br.y);

        if self.dlt.intersects_constraint(&top_left, &top_right)
            || self.dlt.intersects_constraint(&top_right, &bottom_right)
            || self.dlt.intersects_constraint(&bottom_right, &bottom_left)
            || self.dlt.intersects_constraint(&bottom_left, &top_left)
        {
            return None;
        }

        {
            let top_left = self.dlt.insert(top_left);
            let top_right = self.dlt.insert(top_right);
            let bottom_left = self.dlt.insert(bottom_left);
            let bottom_right = self.dlt.insert(bottom_right);

            self.dlt.add_constraint(top_left, top_right);
            self.dlt.add_constraint(bottom_left, bottom_right);
            self.dlt.add_constraint(top_left, bottom_left);
            self.dlt.add_constraint(top_right, bottom_right);
        }

        self.updated_this_tick = true;

        Some(MapHandle {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        })
    }

    pub fn remove(&mut self, handle: &MapHandle) {
        self.dlt.locate_and_remove(&handle.bottom_right);
        self.dlt.locate_and_remove(&handle.bottom_left);
        self.dlt.locate_and_remove(&handle.top_right);
        self.dlt.locate_and_remove(&handle.top_left);
    }

    pub fn impassable_between(&self, a: Vec2, b: Vec2) -> bool {
        self.dlt
            .intersects_constraint(&Point2::new(a.x, a.y), &Point2::new(b.x, b.y))
    }

    pub fn pathfind(
        &self,
        start: Vec2,
        end: Vec2,
        unit_radius: f32,
        debug_triangles: Option<&mut Vec<(Vec2, Vec2)>>,
        debug_funnel_portals: Option<&mut Vec<(Vec2, Vec2)>>,
    ) -> Option<Vec<Vec2>> {
        // If there's nothing between the points then just go straight to the end.
        // This assumes that the unit can fit through all the gaps (all the edges that the line crosses)
        // in between.
        // It'd be better to iterate over all edges that intersect the line and check them against
        // the unit radius.
        if !self.impassable_between(start, end) {
            return Some(vec![end]);
        }

        let start_tri = self.locate(start)?;
        let end_tri = self.locate(end)?;

        // What we do here is we pathfind across the triangulation using each triangles neighbours,
        // and a distance metric that just uses the distance from the triangle centers.
        // Then using this path, we use a funneling algorithm to try and cut across triangles.

        // This doesn't neccessarily give the shortest path though, as the a path with a long
        // triangle center to triangle center distance could have a short funneled distance.
        // A better system would be to iterate over all paths and select the one with the shortest
        // funnel distance.

        // Todo: look into rewriting the implementation based on
        // http://ahamnett.blogspot.com/2012/10/funnel-algorithm.html

        let (triangles, _length) = pathfinding::directed::astar::astar(
            &start_tri,
            |&tri| tri.neighbours(self, unit_radius * 2.0, &end_tri),
            |&tri| OrderedFloat((tri.point - end).mag()),
            |&tri| tri == end_tri,
        )?;

        if let Some(debug_triangles) = debug_triangles {
            debug_triangles.clear();
            debug_triangles.extend(triangles.iter().map(|tri| (tri.center(), tri.point)))
        }

        let funnel_portals = funnel_portals(start, end, unit_radius, &triangles, self);

        if let Some(debug_funnel_portals) = debug_funnel_portals {
            debug_funnel_portals.clear();
            debug_funnel_portals.extend_from_slice(&funnel_portals);
        }

        Some(funnel(&funnel_portals))
    }

    fn offset_by_normal(&self, vertex: Vertex, offset: f32) -> Vec2 {
        // Sum up the lengths of all constraint edges that connect to the vertex
        let sum = vertex
            .ccw_out_edges()
            .filter(|edge| self.dlt.is_constraint_edge(edge.fix()))
            .fold(cgmath::Point2::new(0.0, 0.0), |normal, edge| {
                let edge_delta = *edge.from() - *edge.to();
                normal + edge_delta
            });

        // Normalize them into a normal pointing away from the edge.
        let normal = point_to_vec2(sum).normalized();

        point_to_vec2(*vertex) + (normal * offset)
    }
}

// Construct the 'portals' for a funnel.
// This funnel is a set of left and right points that are esseentially the range of where a path
// could go.
fn funnel_portals(
    start: Vec2,
    end: Vec2,
    unit_radius: f32,
    triangles: &[TriangleRef],
    map: &Map,
) -> Vec<(Vec2, Vec2)> {
    let mut portals = Vec::new();

    // Push the starting point
    portals.push((start, start));

    // Find the edge between the first and second triangles.
    let (mut latest_left, mut latest_right) = triangles[0].shared_edge(&triangles[1]).unwrap();

    // Push those points, but with an offset decided by the unit radius.
    portals.push((
        map.offset_by_normal(latest_left, unit_radius),
        map.offset_by_normal(latest_right, unit_radius),
    ));

    // Push all the middle points
    for i in 1..triangles.len() - 1 {
        let new_point = triangles[i]
            .opposite_point(latest_left, latest_right)
            .unwrap();

        if triangles[i + 1].contains(latest_left) {
            latest_right = new_point;
        } else {
            latest_left = new_point;
        }

        portals.push((
            map.offset_by_normal(latest_left, unit_radius),
            map.offset_by_normal(latest_right, unit_radius),
        ));
    }

    // Push the end point.
    portals.push((end, end));

    portals
}

fn triarea2(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let ax = b.x - a.x;
    let ay = b.y - a.y;
    let bx = c.x - a.x;
    let by = c.y - a.y;

    let area = bx * ay - ax * by;
    // We need to invert this for some reason.
    -area
}

pub fn funnel(portals: &[(Vec2, Vec2)]) -> Vec<Vec2> {
    // Implementation of the Simple Stupid Funnel Algorithm
    // http://digestingduck.blogspot.com/2010/03/simple-stupid-funnel-algorithm.html
    let (mut portal_left, mut portal_right) = portals[0];
    let mut portal_apex = portal_left;

    let mut points = vec![];

    let mut left_index = 0;
    let mut right_index = 0;

    let mut i = 1;

    while i < portals.len() {
        let (left, right) = portals[i];

        // Update right vertex
        if triarea2(portal_apex, portal_right, right) <= 0.0 {
            if portal_apex == portal_right || triarea2(portal_apex, portal_left, right) > 0.0 {
                // Tighten the funnel
                portal_right = right;
                right_index = i;
            } else {
                // Right over left, insert left to path and restart scan from portal left point.
                points.push(portal_left);

                // Make current left the new apex
                portal_apex = portal_left;
                let apex_index = left_index;

                // Reset portal
                portal_left = portal_apex;
                portal_right = portal_apex;
                left_index = apex_index;
                right_index = apex_index;

                // Reset scan
                i = apex_index + 1;
                continue;
            }
        }

        // Update left vertex
        if triarea2(portal_apex, portal_left, left) >= 0.0 {
            if portal_apex == portal_left || triarea2(portal_apex, portal_right, left) < 0.0 {
                // Tighten the funnel
                portal_left = left;
                left_index = i;
            } else {
                // Left over right, insert right to path and restart scan from portal right point.
                points.push(portal_right);

                // Make current right the new apex
                portal_apex = portal_right;
                let apex_index = right_index;

                // Reset portal
                portal_left = portal_apex;
                portal_right = portal_apex;
                left_index = apex_index;
                right_index = apex_index;

                // Reset scan
                i = apex_index + 1;
                continue;
            }
        }

        i += 1;
    }

    let end_point = portals[portals.len() - 1].0;

    if points[points.len() - 1] != end_point {
        points.push(end_point);
    }

    points
}

fn point_to_vec2(point: Point2<f32>) -> Vec2 {
    Vec2::new(point.x, point.y)
}

type Vertex<'a> = VertexHandle<'a, Point2<f32>, CdtEdge>;

#[derive(Debug, Clone, Copy, PartialEq)]
struct TriangleRef<'a> {
    a: Vertex<'a>,
    b: Vertex<'a>,
    c: Vertex<'a>,
    point: Vec2,
}

impl<'a> TriangleRef<'a> {
    fn new(face: FaceHandle<'a, Point2<f32>, CdtEdge>, point: Vec2) -> Self {
        let [a, b, c] = face.as_triangle();
        Self { a, b, c, point }
    }

    fn points(&self) -> [Vec2; 3] {
        [
            point_to_vec2(*self.a),
            point_to_vec2(*self.b),
            point_to_vec2(*self.c),
        ]
    }

    fn center(&self) -> Vec2 {
        Vec2::new(
            self.a.x + self.b.x + self.c.x,
            self.a.y + self.b.y + self.c.y,
        ) / 3.0
    }

    fn neighbours<'b>(
        &self,
        map: &'b Map,
        gap: f32,
        end_tri: &'b Self,
    ) -> impl Iterator<Item = (TriangleRef<'b>, OrderedFloat<f32>)> + 'b {
        let this = *self;

        arrayvec::ArrayVec::from([(this.a, this.b), (this.b, this.c), (this.c, this.a)])
            .into_iter()
            .filter_map(move |(a, b)| {
                // Flipped here because we want the edge facing outside.
                let edge = map.dlt.get_edge_from_neighbors(b.fix(), a.fix()).unwrap();

                let a = point_to_vec2(*a);
                let b = point_to_vec2(*b);

                let face = edge.face();

                if !map.dlt.is_constraint_edge(edge.fix())
                    && gap.powi(2) <= (a - b).mag_sq()
                    && face != map.dlt.infinite_face()
                {
                    // Return a triangle with the 'focus point' set to zero.
                    Some(TriangleRef::new(face, Vec2::zero()))
                } else {
                    None
                }
            })
            // Iterate over all 3 corners and the center and return triangles set with that as the focus point.
            .flat_map(move |triangle| {
                let center = triangle.center();
                arrayvec::ArrayVec::from(triangle.points())
                    .into_iter()
                    .chain(std::iter::once(center))
                    .map(move |point| {
                        let mut tri = triangle;
                        tri.point = point;
                        let dist = (this.point - tri.point).mag();
                        (tri, OrderedFloat(dist))
                    })
                    // If the triangle is the end triangle, add that.
                    .chain({
                        std::iter::once(()).filter_map(move |_| {
                            if triangle.a == end_tri.a
                                && triangle.b == end_tri.b
                                && triangle.c == end_tri.c
                            {
                                let distance = (this.point - end_tri.point).mag();
                                Some((*end_tri, OrderedFloat(distance)))
                            } else {
                                None
                            }
                        })
                    })
            })
    }

    fn contains(&self, point: Vertex) -> bool {
        self.a == point || self.b == point || self.c == point
    }

    fn shared_edge(&self, other: &Self) -> Option<(Vertex, Vertex)> {
        for (a, b) in [(self.a, self.b), (self.b, self.c), (self.c, self.a)].iter() {
            if other.contains(*a) && other.contains(*b) {
                return Some((*a, *b));
            }
        }

        None
    }

    fn opposite_point(&self, a: Vertex, b: Vertex) -> Option<Vertex> {
        for point in [self.a, self.b, self.c].iter() {
            if *point != a && *point != b {
                return Some(*point);
            }
        }

        None
    }
}

impl<'a> Eq for TriangleRef<'a> {}

impl<'a> Hash for TriangleRef<'a> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hash_point(*self.a, hasher);
        hash_point(*self.b, hasher);
        hash_point(*self.c, hasher);
        ordered_float::OrderedFloat(self.point.x).hash(hasher);
        ordered_float::OrderedFloat(self.point.y).hash(hasher);
    }
}

fn hash_point<H: Hasher>(point: Point2<f32>, hasher: &mut H) {
    ordered_float::OrderedFloat(point.x).hash(hasher);
    ordered_float::OrderedFloat(point.y).hash(hasher);
}
