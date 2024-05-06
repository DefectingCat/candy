use crate::error::{Error, Result};

use crate::config::{HostRouteMap, SettingRoute};

/// Parse assets file path
///
/// ## Arguments
///
/// `assets_path`: the rest part of client request path
/// `assets_root`: local directory path from config file
/// `index_file`: index file format from config file
#[inline]
pub fn parse_assets_path(assets_path: &str, assets_root: &str, index_file: &str) -> String {
    match assets_path {
        str if str.ends_with('/') => {
            format!("{}{}{}", assets_root, assets_path, index_file)
        }
        str if str.contains('.') && !str.starts_with('/') => {
            format!("{}/{}", assets_root, assets_path)
        }
        str if !str.starts_with('/') => {
            format!("{}/{}{}", assets_root, assets_path, index_file)
        }
        _ => {
            format!("{}{}/{}", assets_root, assets_path, index_file)
        }
    }
}

/// Find target route by req path
///
/// ## Arguments
///
/// `req_path`: client request path
/// `route_map`: router map from config
///
/// ## Return
///
/// a result. return none when path not registried
/// `router`: host from config file
/// `assets_path`: the rest part of client request path
pub fn find_route<'a>(
    req_path: &'a str,
    route_map: &'a HostRouteMap,
) -> Result<(&'a SettingRoute, &'a str)> {
    let mut index = 1;
    let len = req_path.len();
    let not_found_err = format!("resource {} not found", &req_path);
    let (router, assets_path) = loop {
        if index > len {
            return Err(Error::NotFound(not_found_err));
        }
        let check_path = &req_path[..index];
        match route_map.get(check_path) {
            Some(router) => break (router, &req_path[index..]),
            None => {
                index += 1;
            }
        }
    };
    Ok((router, assets_path))
}
