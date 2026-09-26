use std::collections::HashSet;

use serde::{Deserialize, Serialize};

const MAX_PLAYLISTS: usize = 100;
const MAX_PLAYLIST_ITEMS: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub items: Vec<PlaylistItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistSession {
    pub playlists: Vec<Playlist>,
    #[serde(rename = "activePlaylistId")]
    pub active_playlist_id: Option<String>,
}

pub fn sanitize(items: Vec<PlaylistItem>) -> Vec<PlaylistItem> {
    let mut result = Vec::with_capacity(items.len().min(MAX_PLAYLIST_ITEMS));
    let mut paths = HashSet::with_capacity(items.len().min(MAX_PLAYLIST_ITEMS));
    let mut ids = HashSet::with_capacity(items.len().min(MAX_PLAYLIST_ITEMS));

    for item in items {
        if item.id.trim().is_empty()
            || item.path.trim().is_empty()
            || item.name.trim().is_empty()
            || paths.contains(&item.path)
            || ids.contains(&item.id)
        {
            continue;
        }
        paths.insert(item.path.clone());
        ids.insert(item.id.clone());
        result.push(item);
        if result.len() == MAX_PLAYLIST_ITEMS {
            break;
        }
    }

    result
}

pub fn sanitize_session(session: PlaylistSession) -> PlaylistSession {
    let mut playlists = Vec::with_capacity(session.playlists.len().min(MAX_PLAYLISTS));
    let mut playlist_ids = HashSet::new();
    let mut item_ids = HashSet::new();

    for mut playlist in session.playlists {
        if playlist.id.trim().is_empty()
            || playlist.name.trim().is_empty()
            || playlist_ids.contains(&playlist.id)
            || playlists.len() >= MAX_PLAYLISTS
        {
            continue;
        }

        playlist.items = sanitize(playlist.items)
            .into_iter()
            .filter(|item| item_ids.insert(item.id.clone()))
            .collect();
        playlist_ids.insert(playlist.id.clone());
        playlists.push(playlist);
    }

    let active_playlist_id = session
        .active_playlist_id
        .filter(|id| playlists.iter().any(|playlist| &playlist.id == id));

    PlaylistSession {
        playlists,
        active_playlist_id,
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize, sanitize_session, Playlist, PlaylistItem, PlaylistSession};

    fn item(id: &str, path: &str, name: &str) -> PlaylistItem {
        PlaylistItem {
            id: id.into(),
            path: path.into(),
            name: name.into(),
        }
    }

    fn playlist(id: &str, name: &str, items: Vec<PlaylistItem>) -> Playlist {
        Playlist {
            id: id.into(),
            name: name.into(),
            items,
        }
    }

    #[test]
    fn removes_empty_and_duplicate_paths() {
        let result = sanitize(vec![
            item("1", "/video/a.mp4", "a.mp4"),
            item("2", "/video/a.mp4", "duplicate.mp4"),
            item("3", "", "missing path"),
            item("4", "/video/b.mkv", "b.mkv"),
        ]);

        assert_eq!(
            result,
            vec![
                item("1", "/video/a.mp4", "a.mp4"),
                item("4", "/video/b.mkv", "b.mkv")
            ]
        );
    }

    #[test]
    fn removes_empty_and_duplicate_ids() {
        let result = sanitize(vec![
            item("1", "/video/a.mp4", "a.mp4"),
            item("1", "/video/b.mp4", "b.mp4"),
            item("", "/video/c.mp4", "c.mp4"),
            item("4", "/video/d.mp4", "d.mp4"),
        ]);

        assert_eq!(
            result,
            vec![
                item("1", "/video/a.mp4", "a.mp4"),
                item("4", "/video/d.mp4", "d.mp4")
            ]
        );
    }

    #[test]
    fn sanitizes_each_playlist_and_keeps_the_active_playlist() {
        let result = sanitize_session(PlaylistSession {
            playlists: vec![
                playlist(
                    "favorites",
                    "Favorites",
                    vec![item("1", "/video/a.mp4", "a.mp4")],
                ),
                playlist("nasyid", "Nasyid", vec![item("2", "/video/b.mp4", "b.mp4")]),
            ],
            active_playlist_id: Some("nasyid".into()),
        });

        assert_eq!(result.playlists.len(), 2);
        assert_eq!(result.playlists[0].items.len(), 1);
        assert_eq!(result.active_playlist_id.as_deref(), Some("nasyid"));
    }

    #[test]
    fn caps_session_size_without_reordering_items() {
        let items = (0..2_005)
            .map(|index| item(&index.to_string(), &format!("/video/{index}.mp4"), "video"))
            .collect();

        let result = sanitize_session(PlaylistSession {
            playlists: vec![playlist("one", "One", items)],
            active_playlist_id: Some("one".into()),
        });
        assert_eq!(result.playlists[0].items.len(), 2_000);
        assert_eq!(
            result.playlists[0]
                .items
                .first()
                .map(|entry| entry.id.as_str()),
            Some("0")
        );
        assert_eq!(
            result.playlists[0]
                .items
                .last()
                .map(|entry| entry.id.as_str()),
            Some("1999")
        );
    }
}
