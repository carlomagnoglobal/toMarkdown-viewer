/**
 * Tab Manager
 * Handles tab rendering, keyboard shortcuts, and context menus
 */

class TabManager {
  constructor() {
    this.tabs = [];
    this.activeTab = null;
    this.closedTabs = []; // For reopen functionality
    this.tabBar = document.getElementById('tabbar');
    this.contextMenu = null;
    this.isInitialized = false;

    // Get Tauri invoke function
    this.invoke = window.__TAURI__.core.invoke;

    // Bind methods
    this.render = this.render.bind(this);
    this.handleKeyboard = this.handleKeyboard.bind(this);
    this.showContextMenu = this.showContextMenu.bind(this);
    this.hideContextMenu = this.hideContextMenu.bind(this);

    // Initialize on DOM ready
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', () => this.init());
    } else {
      this.init();
    }
  }

  /**
   * Initialize tab manager
   */
  async init() {
    if (this.isInitialized) return;
    this.isInitialized = true;

    // Wait for Tauri to be available if not already
    if (!window.__TAURI__) {
      console.warn('[TabManager] Tauri not available yet, waiting...');
      await new Promise(resolve => {
        const checkTauri = () => {
          if (window.__TAURI__) {
            resolve();
          } else {
            setTimeout(checkTauri, 100);
          }
        };
        checkTauri();
      });
    }

    // Create context menu element
    this.createContextMenuTemplate();

    // Load initial tabs from backend
    await this.loadTabs();

    // Set up keyboard handlers
    document.addEventListener('keydown', this.handleKeyboard);

    // Set up click handlers for tab bar
    this.tabBar.addEventListener('click', (e) => this.handleTabClick(e));
    this.tabBar.addEventListener('contextmenu', (e) => this.handleContextMenu(e));

    // Hide context menu on click outside
    document.addEventListener('click', (e) => {
      if (e.target !== this.contextMenu && !this.contextMenu?.contains(e.target)) {
        this.hideContextMenu();
      }
    });

    console.log('[TabManager] Initialized');
  }

  /**
   * Load tabs from backend via get_tabs command
   */
  async loadTabs() {
    try {
      const response = await this.invoke('get_tabs');
      this.tabs = response.open_tabs || [];
      this.activeTab = response.active_tab;
      this.render();
      console.log('[TabManager] Loaded', this.tabs.length, 'tabs');
    } catch (error) {
      console.error('[TabManager] Failed to load tabs:', error);
    }
  }

  /**
   * Create context menu template
   */
  createContextMenuTemplate() {
    if (!document.getElementById('tab-context-menu')) {
      const menu = document.createElement('div');
      menu.id = 'tab-context-menu';
      menu.className = 'tab-context-menu';
      menu.style.display = 'none';
      menu.innerHTML = `
        <div class="tab-context-item" data-action="close">Close</div>
        <div class="tab-context-item" data-action="closeOthers">Close All Others</div>
        <div class="tab-context-item" data-action="closeRight">Close Tabs to the Right</div>
        <hr style="margin: 4px 0; border: none; border-top: 1px solid var(--border);">
        <div class="tab-context-item" data-action="closeAll">Close All</div>
        <hr style="margin: 4px 0; border: none; border-top: 1px solid var(--border);">
        <div class="tab-context-item" data-action="reopen" id="reopen-item">Reopen</div>
      `;
      document.body.appendChild(menu);
      this.contextMenu = menu;

      // Add event listeners to context menu items
      menu.addEventListener('click', (e) => this.handleContextMenuAction(e));
    }
  }

  /**
   * Render tabs to the tab bar
   */
  render() {
    this.tabBar.innerHTML = '';

    if (this.tabs.length === 0) {
      this.tabBar.style.display = 'none';
      return;
    }

    this.tabBar.style.display = 'flex';

    this.tabs.forEach((tab) => {
      const tabEl = document.createElement('div');
      tabEl.className = 'tab';
      tabEl.dataset.tabId = tab.id;
      if (tab.id === this.activeTab) {
        tabEl.classList.add('active');
      }

      // Tab title with dirty indicator
      const titleEl = document.createElement('span');
      titleEl.className = 't';
      titleEl.textContent = tab.title;
      if (tab.is_dirty) {
        titleEl.textContent = '● ' + tab.title;
        titleEl.classList.add('dirty');
      }
      tabEl.appendChild(titleEl);

      // Close button
      const closeBtn = document.createElement('button');
      closeBtn.className = 'close';
      closeBtn.innerHTML = '✕';
      closeBtn.setAttribute('aria-label', 'Close ' + tab.title);
      closeBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.closeTab(tab.id);
      });
      tabEl.appendChild(closeBtn);

      // Click handler for tab selection
      tabEl.addEventListener('click', () => this.selectTab(tab.id));

      // Context menu on right-click
      tabEl.addEventListener('contextmenu', (e) => this.handleContextMenu(e, tab.id));

      this.tabBar.appendChild(tabEl);
    });
  }

  /**
   * Select a tab
   */
  async selectTab(tabId) {
    try {
      await this.invoke('set_active_tab', { tab_id: tabId });
      this.activeTab = tabId;
      this.render();
      console.log('[TabManager] Selected tab:', tabId);
    } catch (error) {
      console.error('[TabManager] Failed to select tab:', error);
    }
  }

  /**
   * Close a tab
   */
  async closeTab(tabId) {
    try {
      // Store tab info for reopen functionality
      const tab = this.tabs.find((t) => t.id === tabId);
      if (tab) {
        this.closedTabs.push(tab);
        // Keep only last 5 closed tabs
        if (this.closedTabs.length > 5) {
          this.closedTabs.shift();
        }
      }

      await this.invoke('close_tab', { tab_id: tabId });
      this.tabs = this.tabs.filter((t) => t.id !== tabId);

      // If closed tab was active, load fresh state from backend
      if (this.activeTab === tabId) {
        await this.loadTabs();
      } else {
        this.render();
      }

      console.log('[TabManager] Closed tab:', tabId);
    } catch (error) {
      console.error('[TabManager] Failed to close tab:', error);
    }
  }

  /**
   * Close all tabs
   */
  async closeAllTabs() {
    const tabIds = [...this.tabs.map((t) => t.id)];
    for (const tabId of tabIds) {
      await this.closeTab(tabId);
    }
  }

  /**
   * Handle keyboard shortcuts
   */
  handleKeyboard(e) {
    const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const cmdKey = isMac ? e.metaKey : e.ctrlKey;

    // Cmd+Tab / Ctrl+Tab: Next tab
    if (cmdKey && e.key === 'Tab' && !e.shiftKey) {
      e.preventDefault();
      this.cycleTabForward();
      return;
    }

    // Cmd+Shift+Tab / Ctrl+Shift+Tab: Previous tab
    if (cmdKey && e.key === 'Tab' && e.shiftKey) {
      e.preventDefault();
      this.cycleTabBackward();
      return;
    }

    // Cmd+W / Ctrl+W: Close current tab
    if (cmdKey && e.key === 'w') {
      e.preventDefault();
      if (this.activeTab) {
        this.closeTab(this.activeTab);
      }
      return;
    }

    // Cmd+1/2/3... / Ctrl+1/2/3...: Jump to tab by number
    if (cmdKey && e.key >= '1' && e.key <= '9') {
      e.preventDefault();
      const index = parseInt(e.key) - 1;
      if (index < this.tabs.length) {
        this.selectTab(this.tabs[index].id);
      }
      return;
    }

    // Cmd+[ / Ctrl+[: Previous tab
    if (cmdKey && (e.key === '[' || e.key === '{')) {
      e.preventDefault();
      this.cycleTabBackward();
      return;
    }

    // Cmd+] / Ctrl+]: Next tab
    if (cmdKey && (e.key === ']' || e.key === '}')) {
      e.preventDefault();
      this.cycleTabForward();
      return;
    }

    // Cmd+Shift+T: Reopen last closed tab
    if (cmdKey && e.shiftKey && (e.key === 't' || e.key === 'T')) {
      e.preventDefault();
      this.reopenLastClosedTab();
      return;
    }
  }

  /**
   * Cycle to next tab
   */
  cycleTabForward() {
    if (this.tabs.length === 0) return;

    const currentIndex = this.tabs.findIndex((t) => t.id === this.activeTab);
    const nextIndex = (currentIndex + 1) % this.tabs.length;
    this.selectTab(this.tabs[nextIndex].id);
  }

  /**
   * Cycle to previous tab
   */
  cycleTabBackward() {
    if (this.tabs.length === 0) return;

    const currentIndex = this.tabs.findIndex((t) => t.id === this.activeTab);
    const prevIndex = currentIndex === 0 ? this.tabs.length - 1 : currentIndex - 1;
    this.selectTab(this.tabs[prevIndex].id);
  }

  /**
   * Reopen last closed tab
   */
  async reopenLastClosedTab() {
    if (this.closedTabs.length === 0) {
      console.log('[TabManager] No closed tabs to reopen');
      return;
    }

    const tab = this.closedTabs.pop();
    try {
      await this.invoke('add_tab', {
        path: tab.path,
        title: tab.title,
        tab_type: tab.tab_type,
      });
      await this.loadTabs();
      console.log('[TabManager] Reopened tab:', tab.id);
    } catch (error) {
      console.error('[TabManager] Failed to reopen tab:', error);
    }
  }

  /**
   * Handle tab click
   */
  handleTabClick(e) {
    if (e.target.classList.contains('close')) {
      return; // Handled by close button listener
    }

    const tabEl = e.target.closest('.tab');
    if (tabEl) {
      const tabId = tabEl.dataset.tabId;
      this.selectTab(tabId);
    }
  }

  /**
   * Handle right-click context menu
   */
  handleContextMenu(e, tabId = null) {
    e.preventDefault();

    // Get tab ID from right-clicked element if not provided
    if (!tabId) {
      const tabEl = e.target.closest('.tab');
      if (tabEl) {
        tabId = tabEl.dataset.tabId;
      }
    }

    this.currentContextMenuTabId = tabId;

    // Update context menu state
    const reopenItem = this.contextMenu.querySelector('#reopen-item');
    if (reopenItem) {
      reopenItem.style.display = this.closedTabs.length > 0 ? 'block' : 'none';
    }

    // Show context menu at cursor position
    this.contextMenu.style.display = 'block';
    this.contextMenu.style.left = e.clientX + 'px';
    this.contextMenu.style.top = e.clientY + 'px';

    // Adjust position if menu goes off-screen
    setTimeout(() => {
      const rect = this.contextMenu.getBoundingClientRect();
      if (rect.right > window.innerWidth) {
        this.contextMenu.style.left = (e.clientX - rect.width) + 'px';
      }
      if (rect.bottom > window.innerHeight) {
        this.contextMenu.style.top = (e.clientY - rect.height) + 'px';
      }
    }, 0);
  }

  /**
   * Handle context menu actions
   */
  async handleContextMenuAction(e) {
    const item = e.target.closest('.tab-context-item');
    if (!item) return;

    const action = item.dataset.action;
    const tabId = this.currentContextMenuTabId;

    this.hideContextMenu();

    switch (action) {
      case 'close':
        if (tabId) {
          await this.closeTab(tabId);
        }
        break;

      case 'closeOthers':
        if (tabId) {
          const otherTabIds = this.tabs
            .filter((t) => t.id !== tabId)
            .map((t) => t.id);
          for (const id of otherTabIds) {
            await this.closeTab(id);
          }
        }
        break;

      case 'closeRight':
        if (tabId) {
          const tabIndex = this.tabs.findIndex((t) => t.id === tabId);
          const rightTabIds = this.tabs
            .slice(tabIndex + 1)
            .map((t) => t.id);
          for (const id of rightTabIds) {
            await this.closeTab(id);
          }
        }
        break;

      case 'closeAll':
        await this.closeAllTabs();
        break;

      case 'reopen':
        await this.reopenLastClosedTab();
        break;
    }
  }

  /**
   * Hide context menu
   */
  hideContextMenu() {
    if (this.contextMenu) {
      this.contextMenu.style.display = 'none';
    }
  }

  /**
   * Add a new tab (external API)
   */
  async addTab(path, title, tabType) {
    try {
      const tabId = await this.invoke('add_tab', { path, title, tab_type: tabType });
      await this.loadTabs();
      return tabId;
    } catch (error) {
      console.error('[TabManager] Failed to add tab:', error);
      throw error;
    }
  }

  /**
   * Navigate back to previous tab
   */
  async goBack() {
    try {
      await this.invoke('back_button');
      await this.loadTabs();
    } catch (error) {
      console.error('[TabManager] Failed to go back:', error);
    }
  }
}

// Initialize on page load
const tabManager = new TabManager();

// Export for use in other scripts
window.tabManager = tabManager;
