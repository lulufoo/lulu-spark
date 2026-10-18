import { HomeNavIcon, type HomeNavIconName } from './nav-icons.tsx';

export type SidebarPane = 'home' | 'chat';

const TABS: { pane: SidebarPane; label: string; icon: HomeNavIconName }[] = [
  { pane: 'home', label: 'Home', icon: 'home' },
  { pane: 'chat', label: 'Spark AI', icon: 'spark-ai' },
];

/**
 * The tab row under the sidebar's top chrome: Home and Spark AI as sibling capsule tabs, and the
 * search button at the right edge. The selected tab shows its label; the other is an icon.
 */
export function SidebarTabs({
  pane,
  onChange,
  onSearch,
}: {
  pane: SidebarPane;
  onChange: (pane: SidebarPane) => void;
  onSearch: () => void;
}) {
  return (
    <div className="home-sidebar-tabs" role="tablist" aria-label="Sidebar">
      {TABS.map((tab) => {
        const selected = pane === tab.pane;
        return (
          <button
            key={tab.pane}
            type="button"
            role="tab"
            className={`home-sidebar-tab${selected ? ' is-active' : ''}`}
            data-pane={tab.pane}
            data-tip={tab.label}
            aria-selected={selected}
            onClick={() => onChange(tab.pane)}
          >
            <HomeNavIcon name={tab.icon} />
            <span className="home-sidebar-tab-label">{tab.label}</span>
          </button>
        );
      })}
      <button
        type="button"
        className="home-top-action home-sidebar-search"
        data-role="sidebar-search"
        aria-label="Search"
        data-tip="Search"
        onClick={onSearch}
      >
        <HomeNavIcon name="search" />
      </button>
    </div>
  );
}
