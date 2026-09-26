<script lang="ts">
  import type { Playlist, PlaylistItem, RepeatMode } from "../types";
  import type { RecentFile } from "../lib/recent";

  export let playlists: Playlist[];
  export let activePlaylistId: string;
  export let items: PlaylistItem[];
  export let currentId: string | null;
  export let onPlaylistSelect: (id: string) => void;
  export let onNewPlaylist: () => void;
  export let onRenamePlaylist: (id: string) => void;
  export let onDeletePlaylist: (id: string) => void;
  export let onSelect: (item: PlaylistItem) => void;
  export let onRemove: (id: string) => void;
  export let onMove: (id: string, direction: -1 | 1) => void;
  export let onOpen: () => void;
  export let onClear: () => void;
  export let onClose: () => void;
  export let repeatMode: RepeatMode;
  export let shuffleEnabled: boolean;
  export let onRepeatToggle: () => void;
  export let onShuffleToggle: () => void;
  export let recentFiles: RecentFile[] = [];
  export let recentAvailability: Map<string, boolean> = new Map();
  export let onRecentSelect: (entry: RecentFile) => void;
  export let onRecentAdd: (entry: RecentFile) => void;

  const rowHeight = 35;
  const overscan = 3;
  let viewportHeight = 280;
  let scrollTop = 0;
  let viewportElement: HTMLDivElement | undefined;
  let searchQuery = "";
  let libraryView: "playlists" | "recent" = "playlists";

  $: activePlaylist = playlists.find((playlist) => playlist.id === activePlaylistId) ?? playlists[0];
  $: normalizedSearch = searchQuery.trim().toLowerCase();
  $: filteredItems = normalizedSearch
    ? items.filter((item) => item.name.toLowerCase().includes(normalizedSearch))
    : items;
  $: visibleCount = Math.ceil(viewportHeight / rowHeight) + overscan;
  $: start = Math.max(0, Math.floor(scrollTop / rowHeight) - 1);
  $: visibleItems = filteredItems.slice(start, start + visibleCount);
  $: topPadding = start * rowHeight;
  $: bottomPadding = Math.max(0, (filteredItems.length - start - visibleItems.length) * rowHeight);

  function observeViewport(node: HTMLDivElement): { destroy: () => void } | undefined {
    viewportElement = node;

    const updateViewportHeight = (): void => {
      viewportHeight = Math.max(node.clientHeight, rowHeight);
    };

    updateViewportHeight();
    if (typeof ResizeObserver === "undefined") return undefined;

    const observer = new ResizeObserver(updateViewportHeight);
    observer.observe(node);

    return {
      destroy: () => {
        observer.disconnect();
        if (viewportElement === node) viewportElement = undefined;
      },
    };
  }

  function onScroll(event: Event): void {
    const target = event.currentTarget as HTMLDivElement;
    scrollTop = target.scrollTop;
    viewportHeight = Math.max(target.clientHeight, rowHeight);
  }

  function onSearchInput(event: Event): void {
    searchQuery = (event.currentTarget as HTMLInputElement).value;
    scrollTop = 0;
    if (viewportElement) viewportElement.scrollTop = 0;
  }

  function selectPlaylist(id: string): void {
    searchQuery = "";
    scrollTop = 0;
    if (viewportElement) viewportElement.scrollTop = 0;
    onPlaylistSelect(id);
  }

  function selectLibraryView(view: "playlists" | "recent"): void {
    libraryView = view;
    searchQuery = "";
    scrollTop = 0;
  }
</script>

<aside class="playlist-panel" aria-label="Playlists">
  <div class="panel-heading">
    <div>
      <p class="panel-kicker">MULTI PLAYLIST</p>
      <h2>{activePlaylist?.name ?? "Playlist"} <span>{items.length}</span></h2>
    </div>
    <button class="panel-close" type="button" aria-label="Close playlist drawer" title="Close playlist drawer" onclick={onClose}>×</button>
  </div>

  <section class="playlist-switcher" aria-label="Playlist library">
    <div class="library-tabs" role="tablist" aria-label="Library sections">
      <button class:active={libraryView === "recent"} class="library-tab" role="tab" aria-selected={libraryView === "recent"} onclick={() => selectLibraryView("recent")}>RECENT <span>{recentFiles.length}</span></button>
      <button class:active={libraryView === "playlists"} class="library-tab" role="tab" aria-selected={libraryView === "playlists"} onclick={() => selectLibraryView("playlists")}>PLAYLISTS</button>
    </div>
    {#if libraryView === "playlists"}
      <div class="switcher-label">
        <span>PLAYLISTS</span>
        <button class="new-playlist-button" onclick={onNewPlaylist}>+ NEW PLAYLIST</button>
      </div>
      <div class="playlist-list">
        {#each playlists as playlist (playlist.id)}
          <div class:active={playlist.id === activePlaylistId} class="playlist-choice">
            <button class="playlist-select" aria-current={playlist.id === activePlaylistId ? "true" : undefined} onclick={() => selectPlaylist(playlist.id)}>
              <span class="playlist-name">{playlist.name}</span>
              <span class="playlist-count">{playlist.items.length}</span>
            </button>
            <div class="playlist-actions">
              <button aria-label={`Rename ${playlist.name}`} title="Rename playlist" onclick={() => onRenamePlaylist(playlist.id)}>✎</button>
              <button aria-label={`Delete ${playlist.name}`} title="Delete playlist" onclick={() => onDeletePlaylist(playlist.id)}>×</button>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="recent-list" aria-label="Recent files">
        {#if recentFiles.length === 0}
          <div class="recent-empty">NO RECENT FILES</div>
        {:else}
          {#each recentFiles as entry (entry.path)}
            {@const available = recentAvailability.get(entry.path) ?? true}
            <div class:missing={!available} class="recent-row">
              <button class="recent-main" title={entry.path} onclick={() => onRecentSelect(entry)}>
                <span class="recent-status" aria-hidden="true">{available ? "●" : "⚠"}</span>
                <span class="recent-name">{entry.name}</span>
              </button>
              <button class="recent-add" title="Add to active playlist" aria-label={`Add ${entry.name} to playlist`} onclick={() => onRecentAdd(entry)}>+</button>
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </section>

  <div class="track-heading">
    <div>
      <span class="track-heading-label">TRACKS IN ACTIVE PLAYLIST</span>
      <span class="track-heading-count">{normalizedSearch ? `${filteredItems.length}/${items.length}` : items.length}</span>
    </div>
    <div class="track-actions">
      <div class="playback-mode-controls" aria-label="Playlist playback modes">
        <button
          class:active={repeatMode !== "off"}
          class="mode-button"
          type="button"
          aria-label={`Repeat ${repeatMode === "off" ? "off" : repeatMode === "one" ? "one" : "all"}`}
          aria-pressed={repeatMode !== "off"}
          title={`Repeat: ${repeatMode === "off" ? "off" : repeatMode === "one" ? "one" : "all"}. Click to cycle.`}
          onclick={onRepeatToggle}
        >↻<span>{repeatMode === "one" ? "1" : repeatMode === "all" ? "ALL" : ""}</span></button>
        <button
          class:active={shuffleEnabled}
          class="mode-button"
          type="button"
          aria-label={`Shuffle ${shuffleEnabled ? "on" : "off"}`}
          aria-pressed={shuffleEnabled}
          title={`Shuffle ${shuffleEnabled ? "on" : "off"}`}
          onclick={onShuffleToggle}
        >⤨</button>
      </div>
      <button class="add-track-button" onclick={onOpen}>+ ADD VIDEO</button>
      <button class="clear-track-button" onclick={onClear}>CLEAR</button>
    </div>
  </div>

  <label class="playlist-search">
    <span aria-hidden="true">⌕</span>
    <input type="search" value={searchQuery} placeholder="SEARCH TRACKS" aria-label="Search tracks" oninput={onSearchInput} />
  </label>

  {#if items.length === 0}
    <button class="playlist-empty" onclick={onOpen}>
      <span>+ ADD VIDEO</span>
      <small>{activePlaylist?.name ?? "Playlist"} is empty</small>
    </button>
  {:else if filteredItems.length === 0}
    <div class="playlist-empty search-empty">
      <span>NO MATCHING TRACKS</span>
      <small>Try another filename</small>
    </div>
  {:else}
    <div class="playlist-viewport" use:observeViewport onscroll={onScroll}>
      <div style={`padding-top: ${topPadding}px; padding-bottom: ${bottomPadding}px`}>
        {#each visibleItems as item, visibleIndex (item.id)}
          <div class:active={item.id === currentId} class="playlist-row" style={`height: ${rowHeight}px`}>
            <button class="item-main" aria-current={item.id === currentId ? "true" : undefined} title={item.path} onclick={() => onSelect(item)}>
              <span class="item-index">{start + visibleIndex + 1}</span>
              <span class="item-name">{item.name}</span>
            </button>
            <div class="item-actions">
              <button aria-label={`Move ${item.name} up`} title="Move up" onclick={() => onMove(item.id, -1)}>↑</button>
              <button aria-label={`Move ${item.name} down`} title="Move down" onclick={() => onMove(item.id, 1)}>↓</button>
              <button aria-label={`Remove ${item.name}`} title="Remove" onclick={() => onRemove(item.id)}>×</button>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <footer class="playlist-footer">
    <span>{activePlaylist?.name ?? "PLAYLIST"}</span>
    <span>↑↓ REORDER</span>
  </footer>
</aside>

<style>
  .playlist-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 236px;
    max-width: 290px;
    overflow: hidden;
    color: var(--skin-text);
    background: var(--skin-surface);
  }

  .panel-heading {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 0.8rem 0.65rem;
    border-bottom: 1px solid var(--skin-border);
  }

  .panel-kicker,
  .switcher-label,
  .track-heading-label {
    color: var(--skin-accent);
    font: 700 0.58rem/1 monospace;
    letter-spacing: 0.16em;
  }

  .panel-kicker {
    margin: 0 0 0.2rem;
  }

  .panel-close {
    display: grid;
    place-items: center;
    width: 23px;
    height: 23px;
    flex: 0 0 auto;
    color: var(--skin-accent-strong);
    background: var(--skin-surface-raised);
    border: 1px solid var(--button-shadow);
    border-color: var(--button-highlight) var(--button-shadow) var(--button-shadow) var(--button-highlight);
    box-shadow: inset 1px 1px 0 var(--button-highlight), inset -1px -1px 0 var(--button-shadow);
    font: 700 0.9rem/1 monospace;
    transition: color 100ms ease, transform 100ms ease, border-color 100ms ease;
  }

  .panel-close:hover,
  .panel-close:focus-visible {
    color: var(--skin-text);
    border-color: var(--skin-accent);
    outline: none;
    transform: scale(1.08);
  }

  .panel-close:active {
    border-color: var(--button-shadow) var(--button-highlight) var(--button-highlight) var(--button-shadow);
    box-shadow: inset 1px 1px 0 var(--button-shadow), inset -1px -1px 0 var(--button-highlight);
    transform: scale(0.98);
  }

  h2 {
    max-width: 10rem;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 700 1.05rem/1 Georgia, serif;
  }

  h2 span,
  .track-heading-count,
  .playlist-count {
    color: var(--skin-text-muted);
    font: 0.68rem/1 monospace;
  }

  button {
    color: inherit;
    border: 0;
    cursor: pointer;
  }

  .new-playlist-button:hover,
  .new-playlist-button:focus-visible {
    color: var(--skin-text);
    border-color: var(--skin-accent);
    outline: none;
  }

  .playlist-switcher {
    flex: 0 0 auto;
    padding: 0.55rem 0.65rem 0.45rem;
    border-bottom: 1px solid var(--skin-border);
  }

  .switcher-label,
  .track-heading,
  .track-heading > div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
  }

  .library-tabs {
    display: flex;
    gap: 0.25rem;
    margin-bottom: 0.45rem;
  }

  .library-tab {
    padding: 0.25rem 0.38rem;
    color: var(--skin-text-muted);
    background: var(--skin-surface-deep);
    border: 1px solid var(--skin-border);
    font: 700 0.57rem/1 monospace;
    letter-spacing: 0.08em;
  }

  .library-tab span {
    color: var(--skin-accent);
    font-size: 0.5rem;
  }

  .library-tab.active,
  .library-tab:hover,
  .library-tab:focus-visible {
    color: var(--skin-accent-contrast);
    background: var(--skin-accent);
    border-color: var(--skin-accent-strong);
    outline: none;
  }

  .recent-list {
    display: grid;
    gap: 2px;
    max-height: 10rem;
    overflow-y: auto;
  }

  .recent-row {
    display: flex;
    min-width: 0;
    border: 1px solid transparent;
  }

  .recent-row:hover {
    background: var(--skin-surface-raised);
    border-color: var(--skin-border);
  }

  .recent-main {
    display: flex;
    align-items: center;
    min-width: 0;
    flex: 1;
    gap: 0.35rem;
    padding: 0.32rem 0.35rem;
    color: var(--skin-text);
    background: transparent;
    text-align: left;
  }

  .recent-status {
    flex: 0 0 auto;
    color: var(--skin-accent);
    font-size: 0.55rem;
  }

  .recent-row.missing .recent-status,
  .recent-row.missing .recent-name {
    color: #bd6b58;
  }

  .recent-name {
    min-width: 0;
    overflow: hidden;
    color: var(--skin-text-muted);
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 0.67rem/1.2 monospace;
  }

  .recent-main:hover,
  .recent-main:focus-visible {
    color: var(--skin-text);
    outline: none;
  }

  .recent-add {
    width: 23px;
    color: var(--skin-accent-strong);
    background: transparent;
    font: 700 0.9rem/1 monospace;
  }

  .recent-add:hover,
  .recent-add:focus-visible {
    color: var(--skin-text);
    outline: none;
  }

  .recent-empty {
    padding: 0.55rem 0.2rem;
    color: var(--skin-text-muted);
    font: 0.58rem/1 monospace;
    letter-spacing: 0.08em;
  }

  .new-playlist-button,
  .add-track-button {
    padding: 0.15rem 0.25rem;
    color: var(--skin-accent-strong);
    background: transparent;
    font: 700 0.58rem/1 monospace;
    letter-spacing: 0.04em;
  }

  .playlist-list {
    display: grid;
    gap: 2px;
    max-height: 10rem;
    margin-top: 0.4rem;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-color: var(--skin-border) var(--skin-surface-deep);
    scrollbar-width: thin;
  }

  .playlist-list::-webkit-scrollbar,
  .playlist-viewport::-webkit-scrollbar {
    width: 6px;
  }

  .playlist-list::-webkit-scrollbar-track,
  .playlist-viewport::-webkit-scrollbar-track {
    background: var(--skin-surface-deep);
  }

  .playlist-list::-webkit-scrollbar-thumb,
  .playlist-viewport::-webkit-scrollbar-thumb {
    background: var(--skin-border);
    border: 1px solid var(--skin-surface-deep);
    border-radius: 3px;
  }

  .playlist-list::-webkit-scrollbar-thumb:hover,
  .playlist-viewport::-webkit-scrollbar-thumb:hover {
    background: var(--skin-accent-strong);
  }

  .playlist-choice {
    display: flex;
    min-width: 0;
    border: 1px solid transparent;
  }

  .playlist-choice.active {
    background: var(--skin-surface-raised);
    border-color: var(--skin-border);
    box-shadow: inset 3px 0 var(--skin-accent);
  }

  .playlist-select {
    display: flex;
    align-items: center;
    min-width: 0;
    flex: 1;
    gap: 0.45rem;
    padding: 0.36rem 0.4rem 0.36rem 0.55rem;
    overflow: hidden;
    color: var(--skin-text-muted);
    background: transparent;
    text-align: left;
  }

  .playlist-choice.active .playlist-select,
  .playlist-select:hover,
  .playlist-select:focus-visible {
    color: var(--skin-text);
    outline: none;
  }

  .playlist-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.72rem;
  }

  .playlist-count {
    margin-left: auto;
    font-size: 0.6rem;
  }

  .playlist-actions {
    display: flex;
    align-items: center;
    gap: 1px;
    padding-right: 0.2rem;
  }

  .playlist-actions button {
    width: 18px;
    height: 20px;
    color: var(--skin-text-muted);
    background: transparent;
    font: 0.76rem/1 monospace;
  }

  .playlist-actions button:hover,
  .playlist-actions button:focus-visible {
    color: var(--skin-accent-strong);
    outline: none;
  }

  .track-heading {
    flex: 0 0 auto;
    padding: 0.55rem 0.7rem 0.35rem;
  }

  .track-heading > div {
    justify-content: flex-start;
  }

  .track-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .playlist-search {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex: 0 0 auto;
    margin: 0 0.7rem 0.45rem;
    padding: 0.24rem 0.35rem;
    color: var(--skin-accent);
    background: var(--skin-surface-deep);
    border: 1px solid var(--skin-border);
    box-shadow: inset 1px 1px 0 var(--skin-shadow), inset -1px -1px 0 var(--skin-surface-raised);
  }

  .playlist-search span {
    font: 700 0.8rem/1 monospace;
  }

  .playlist-search input {
    min-width: 0;
    width: 100%;
    padding: 0;
    color: var(--skin-text);
    background: transparent;
    border: 0;
    outline: none;
    font: 700 0.58rem/1 monospace;
    letter-spacing: 0.08em;
  }

  .playlist-search input::placeholder {
    color: var(--skin-text-muted);
    opacity: 0.8;
  }

  .playlist-search input::-webkit-search-cancel-button {
    filter: grayscale(1) sepia(1) saturate(0.5);
  }

  .playback-mode-controls {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .mode-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 21px;
    padding: 0 0.22rem;
    color: var(--skin-text-muted);
    background: var(--skin-surface-raised);
    border: 1px solid var(--button-shadow);
    border-color: var(--button-highlight) var(--button-shadow) var(--button-shadow) var(--button-highlight);
    border-radius: 999px;
    box-shadow: inset 1px 1px 0 var(--button-highlight), inset -1px -1px 0 var(--button-shadow);
    font: 700 0.78rem/1 monospace;
  }

  .mode-button span {
    margin-left: 1px;
    font-size: 0.46rem;
    letter-spacing: 0;
  }

  .mode-button.active {
    color: var(--skin-accent-contrast);
    background: var(--skin-accent);
    border-color: var(--skin-accent-strong);
  }

  .mode-button:hover,
  .mode-button:focus-visible {
    color: var(--skin-accent-strong);
    border-color: var(--skin-accent);
    outline: none;
  }

  .mode-button.active:hover,
  .mode-button.active:focus-visible {
    color: var(--skin-accent-contrast);
    background: var(--skin-accent-strong);
  }

  .track-heading-label {
    color: var(--skin-text-muted);
    font-size: 0.52rem;
    letter-spacing: 0.08em;
  }

  .clear-track-button {
    padding: 0.15rem 0.25rem;
    color: var(--skin-text-muted);
    background: transparent;
    font: 700 0.58rem/1 monospace;
    letter-spacing: 0.04em;
  }

  .clear-track-button:hover,
  .clear-track-button:focus-visible {
    color: var(--skin-accent-strong);
    outline: none;
  }

  .track-heading-count {
    font-size: 0.6rem;
  }

  .playlist-viewport {
    flex: 1 1 0;
    min-height: 0;
    height: auto;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-color: var(--skin-border) var(--skin-surface-deep);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .playlist-row {
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--skin-border-subtle);
  }

  .item-main {
    display: flex;
    align-items: center;
    min-width: 0;
    flex: 1;
    gap: 0.45rem;
    padding: 0 0.4rem;
    overflow: hidden;
    color: var(--skin-text-muted);
    background: transparent;
    text-align: left;
  }

  .item-main:hover,
  .item-main:focus-visible,
  .playlist-row.active .item-main {
    color: var(--skin-text);
    background: var(--skin-surface-raised);
    outline: none;
  }

  .playlist-row.active .item-main {
    box-shadow: inset 3px 0 var(--skin-accent);
  }

  .item-index {
    width: 20px;
    color: var(--skin-accent);
    font: 0.62rem/1 monospace;
    text-align: right;
  }

  .item-name {
    overflow: hidden;
    font-size: 0.74rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-actions {
    display: flex;
    align-items: center;
    gap: 1px;
    padding-right: 0.25rem;
    background: var(--skin-surface);
  }

  .item-actions button {
    width: 17px;
    height: 20px;
    color: var(--skin-text-muted);
    background: transparent;
    font: 0.72rem/1 monospace;
  }

  .item-actions button:hover,
  .item-actions button:focus-visible {
    color: var(--skin-accent-strong);
    outline: none;
  }

  .playlist-empty {
    display: grid;
    place-items: center;
    gap: 0.4rem;
    min-height: 130px;
    margin: 0.4rem 0.75rem 0.75rem;
    color: var(--skin-accent-strong);
    background: var(--skin-surface-deep);
    border: 1px dashed var(--skin-border);
    font: 700 0.68rem/1 monospace;
  }

  .playlist-empty:hover,
  .playlist-empty:focus-visible {
    border-color: var(--skin-accent);
    outline: none;
  }

  .playlist-empty small {
    color: var(--skin-text-muted);
    font: 0.65rem/1 sans-serif;
  }

  .search-empty {
    flex: 0 0 auto;
    margin-top: 0.4rem;
  }

  .playlist-footer {
    display: flex;
    flex: 0 0 auto;
    justify-content: space-between;
    gap: 0.5rem;
    margin-top: auto;
    padding: 0.55rem 0.7rem;
    color: var(--skin-text-muted);
    border-top: 1px solid var(--skin-border);
    font: 0.52rem/1 monospace;
    letter-spacing: 0.06em;
  }
</style>
