use std::path::PathBuf;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[cfg(not(feature = "parallel"))]
pub fn get_paths(
    files: Vec<PathBuf>,
    out_dir: Option<PathBuf>,
    suffix: Option<String>,
    extension: impl ToString,
    recursive: bool,
) -> impl Iterator<Item = (PathBuf, PathBuf)> {
    let common_path = if recursive {
        get_common_path(&files)
    } else {
        None
    };

    files.into_iter().map(move |path| -> (PathBuf, PathBuf) {
        let file_name = path
            .file_stem()
            .and_then(|f| f.to_str())
            .unwrap_or("optimized_image");

        let mut out_path = match &out_dir {
            Some(dir) => {
                if let Some(common) = &common_path {
                    let relative_path =
                        path.parent().unwrap().strip_prefix(common).unwrap_or(&path);
                    dir.join(relative_path)
                } else {
                    dir.clone()
                }
            }
            None => path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
        };

        if let Some(s) = &suffix {
            out_path.push(format!("{file_name}{s}.{}", extension.to_string()));
        } else {
            out_path.push(format!("{file_name}.{}", extension.to_string()));
        }

        (path, out_path)
    })
}

#[cfg(feature = "parallel")]
pub fn get_paths(
    files: Vec<PathBuf>,
    out_dir: Option<PathBuf>,
    suffix: Option<String>,
    extension: impl ToString + Sync + Send,
    recursive: bool,
) -> impl ParallelIterator<Item = (PathBuf, PathBuf)> {
    let common_path = if recursive {
        get_common_path(&files)
    } else {
        None
    };

    files
        .into_par_iter()
        .map(move |path| -> (PathBuf, PathBuf) {
            let file_name = path
                .file_stem()
                .and_then(|f| f.to_str())
                .unwrap_or("optimized_image");

            let mut out_path = match &out_dir {
                Some(dir) => {
                    if let Some(common) = &common_path {
                        let relative_path =
                            path.parent().unwrap().strip_prefix(common).unwrap_or(&path);
                        dir.join(relative_path)
                    } else {
                        dir.clone()
                    }
                }
                None => path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
            };

            if let Some(s) = &suffix {
                out_path.push(format!("{file_name}{s}.{}", extension.to_string()));
            } else {
                out_path.push(format!("{file_name}.{}", extension.to_string()));
            }

            (path, out_path)
        })
}

fn get_common_path(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    let mut common_path = paths[0].clone();

    for path in paths.iter().skip(1) {
        common_path = common_path
            .iter()
            .zip(path.iter())
            .take_while(|&(a, b)| a == b)
            .map(|(a, _)| a)
            .collect();
    }

    Some(common_path)
}

#[inline]
pub fn collect_files(input: Vec<PathBuf>) -> Vec<PathBuf> {
    #[cfg(windows)]
    #[cfg(not(feature = "parallel"))]
    let input = input.into_iter().flat_map(apply_glob_pattern).collect();

    #[cfg(windows)]
    #[cfg(feature = "parallel")]
    let input = input.into_par_iter().flat_map(apply_glob_pattern).collect();

    input
}

#[cfg(windows)]
fn apply_glob_pattern(path: PathBuf) -> Vec<PathBuf> {
    let matches = path
        .to_str()
        .and_then(|pattern| glob::glob(pattern).ok())
        .map(|paths| paths.flatten().collect::<Vec<_>>());

    match matches {
        Some(paths) if !paths.is_empty() => paths,
        _ => vec![path],
    }
}

#[cfg(test)]
mod tests;
