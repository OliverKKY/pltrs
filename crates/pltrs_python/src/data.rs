use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;

/// Parse a Python object into separate x and y vectors.
///
/// Accepted formats:
/// - A list of `(x, y)` tuples or `[x, y]` lists  →  `[(1.0, 2.0), (3.0, 4.0)]`
/// - A list of scalars (1-D)  →  x is inferred as `[0, 1, 2, ...]`
///
/// Returns `(xs, ys)` where both are `Vec<f64>`.
pub fn parse_data(obj: &Bound<'_, PyAny>) -> PyResult<(Vec<f64>, Vec<f64>)> {
    // Try to iterate — everything we accept is iterable.
    let iter = PyIterator::from_object(obj)
        .map_err(|_| PyValueError::new_err("data must be an iterable (list, tuple, or array)"))?;

    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut is_2d: Option<bool> = None;

    for (i, item) in iter.enumerate() {
        let item = item?;

        // Probe first element to decide 1-D vs 2-D.
        if is_2d.is_none() {
            is_2d = Some(is_pair(&item));
        }

        if is_2d == Some(true) {
            let (x, y) = extract_pair(&item)?;
            xs.push(x);
            ys.push(y);
        } else {
            let y: f64 = item
                .extract()
                .map_err(|_| PyValueError::new_err("data elements must be numeric"))?;
            xs.push(i as f64);
            ys.push(y);
        }
    }

    if xs.is_empty() {
        return Err(PyValueError::new_err("data must not be empty"));
    }

    Ok((xs, ys))
}

/// Check whether an element looks like a 2-element sequence (tuple or list).
fn is_pair(obj: &Bound<'_, PyAny>) -> bool {
    if let Ok(len) = obj.len() {
        if len == 2 {
            // Make sure it's not just a string of length 2.
            return !obj.is_instance_of::<pyo3::types::PyString>();
        }
    }
    false
}

/// Extract a `(f64, f64)` pair from a 2-element sequence.
fn extract_pair(obj: &Bound<'_, PyAny>) -> PyResult<(f64, f64)> {
    let x: f64 = obj
        .get_item(0)?
        .extract()
        .map_err(|_| PyValueError::new_err("pair x-value must be numeric"))?;
    let y: f64 = obj
        .get_item(1)?
        .extract()
        .map_err(|_| PyValueError::new_err("pair y-value must be numeric"))?;
    Ok((x, y))
}

/// Compute `(min, max)` limits from a slice of values, with optional padding.
///
/// If `padding` is `0.05`, limits are expanded by 5 % of the range on each side.
/// If all values are identical, returns `(value - 1.0, value + 1.0)`.
pub fn compute_limits(vals: &[f64], padding: f64) -> (f64, f64) {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in vals {
        if v < lo {
            lo = v;
        }
        if v > hi {
            hi = v;
        }
    }

    let range = hi - lo;
    if range.abs() < f64::EPSILON {
        return (lo - 1.0, hi + 1.0);
    }

    let pad = range * padding;
    (lo - pad, hi + pad)
}
