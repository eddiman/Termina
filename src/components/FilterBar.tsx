import { searchQuery, selectedTags, allTags } from '../lib/store';

export function FilterBar() {
  const tags = allTags.value;
  const active = selectedTags.value;

  const toggleTag = (tag: string) => {
    if (active.includes(tag)) {
      selectedTags.value = active.filter(t => t !== tag);
    } else {
      selectedTags.value = [...active, tag];
    }
  };

  return (
    <div class="filter-bar">
      <div class="search-bar">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          type="text"
          class="search-input"
          placeholder="Search commands..."
          value={searchQuery.value}
          onInput={(e) => {
            searchQuery.value = (e.target as HTMLInputElement).value;
          }}
        />
      </div>
      {tags.length > 0 && (
        <div class="tag-filter-row">
          {tags.map(tag => (
            <button
              key={tag}
              class={`tag-filter-chip ${active.includes(tag) ? 'active' : ''}`}
              onClick={() => toggleTag(tag)}
            >
              {tag}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
