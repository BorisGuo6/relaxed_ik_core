use nalgebra::{Vector3};

pub fn point_to_segment_distance(point: &Vector3<f64>, start: &Vector3<f64>, end: &Vector3<f64>) -> f64 {
    let segment = end - start;
    let segment_len_sq = segment.norm_squared();

    if segment_len_sq == 0.0 {
        return (*start - *point).norm();
    }

    // projection = start + ((point - start) · segment) / |segment|^2 * segment
    let t = (point - start).dot(&segment) / segment_len_sq;
    if t < 0.0 {
        // projection is closer to start
        return (*start - *point).norm();
    } else if t > 1.0 {
        // projection is closer to end
        return (*end - *point).norm();
    }

    // projection is in segment
    let projection = start + t * segment;
    (projection - *point).norm()
}