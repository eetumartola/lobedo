#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
const PARALLEL_THRESHOLD: usize = 1024;

pub fn for_each_indexed_mut<T, F>(slice: &mut [T], f: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync + Send,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        if slice.len() >= PARALLEL_THRESHOLD {
            slice
                .par_iter_mut()
                .enumerate()
                .for_each(|(idx, value)| f(idx, value));
            return;
        }
    }

    for (idx, value) in slice.iter_mut().enumerate() {
        f(idx, value);
    }
}

pub fn try_for_each_indexed_mut<T, E, F>(slice: &mut [T], f: F) -> Result<(), E>
where
    T: Send,
    E: Send,
    F: Fn(usize, &mut T) -> Result<(), E> + Sync + Send,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        if slice.len() >= PARALLEL_THRESHOLD {
            return slice
                .par_iter_mut()
                .enumerate()
                .try_for_each(|(idx, value)| f(idx, value));
        }
    }

    for (idx, value) in slice.iter_mut().enumerate() {
        f(idx, value)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn for_each_index<F>(len: usize, f: F)
where
    F: Fn(usize) + Sync + Send,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        if len >= PARALLEL_THRESHOLD {
            (0..len).into_par_iter().for_each(&f);
            return;
        }
    }

    for idx in 0..len {
        f(idx);
    }
}
