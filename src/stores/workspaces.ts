import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import type { ComputedRef } from 'vue';
import { computed, ref } from 'vue';
import type { Tab, TabGroup, Workspace } from '@/types/workspaces';
import uniqueId from '@/utils/unique-id';

export const useWorkspacesStore = defineStore('workspaces', () => {
	const workspaces = ref<Workspace[]>([
		{
			id: 0,
			isPrimary: true,
			isCurrent: true,
			name: 'primary',
			actions: [],
			tabGroups: [],
			currentTabGroupIndex: 0,
			currentTabIndex: 0,
		},
	]);
	const tabCount = ref(0);

	const primaryWorkspace: ComputedRef<Workspace | null> = computed(
		() => workspaces.value?.find((workspace) => workspace.isPrimary) || null
	);

	const currentWorkspace: ComputedRef<Workspace | null> = computed(
		() => workspaces.value?.find((workspace) => workspace.isCurrent) || null
	);

	const currentTabGroup: ComputedRef<TabGroup | null> = computed(
		() =>
			currentWorkspace?.value?.tabGroups[currentWorkspace?.value?.currentTabGroupIndex || 0] || null
	);

	const currentTab: ComputedRef<Tab | null> = computed(() => {
		const tabGroup = currentTabGroup?.value;
		if (!tabGroup?.length) {
			return null;
		}

		const index = currentWorkspace?.value?.currentTabIndex ?? 0;
		return tabGroup[index] ?? tabGroup[0] ?? null;
	});

	const tabs: ComputedRef<Tab[]> = computed(() => currentWorkspace.value?.tabGroups?.flat() || []);

	function createNewTerminalTab(): Tab {
		tabCount.value += 1;
		const name = `Terminal ${tabCount.value}`;

		return {
			id: uniqueId(),
			name,
			path: name,
			type: 'directory',
			paneWidth: 100,
			filterQuery: '',
		};
	}

	function ensureCurrentWorkspace() {
		if (!currentWorkspace.value) {
			const firstWorkspace = workspaces.value[0];
			if (firstWorkspace) {
				firstWorkspace.isCurrent = true;
			}
		}
	}

	function ensureAtLeastOneTabGroup() {
		ensureCurrentWorkspace();
		const workspace = currentWorkspace.value;

		if (!workspace) {
			return;
		}

		if (workspace.tabGroups.length === 0) {
			workspace.tabGroups = [[createNewTerminalTab()]];
			workspace.currentTabGroupIndex = 0;
			workspace.currentTabIndex = 0;
		}
	}

	async function closeSessionByTabId(tabId: string) {
		try {
			await invoke('async_close_shell', { sessionId: tabId });
		} catch (error) {
			console.error('Failed to close terminal session:', error);
		}
	}

	async function closeTabGroupSessions(tabGroup: TabGroup) {
		await Promise.all(tabGroup.map((tab) => closeSessionByTabId(tab.id)));
	}

	async function closeTabGroup(tabGroup: Tab[]) {
		const workspace = currentWorkspace.value;

		if (!workspace) {
			return;
		}

		const closingIndex = workspace.tabGroups.findIndex((group) => group[0]?.id === tabGroup[0]?.id);

		if (closingIndex === -1) {
			return;
		}

		const [removedGroup] = workspace.tabGroups.splice(closingIndex, 1);
		if (removedGroup?.length) {
			await closeTabGroupSessions(removedGroup);
		}

		if (workspace.tabGroups.length === 0) {
			ensureAtLeastOneTabGroup();
			return;
		}

		if (workspace.currentTabGroupIndex >= workspace.tabGroups.length) {
			workspace.currentTabGroupIndex = workspace.tabGroups.length - 1;
		}

		workspace.currentTabIndex = 0;
	}

	async function closeAllTabGroups() {
		const workspace = currentWorkspace.value;

		if (!workspace) {
			return;
		}

		const groupsToClose = workspace.tabGroups;
		await Promise.all(groupsToClose.map((group) => closeTabGroupSessions(group)));

		workspace.tabGroups = [[createNewTerminalTab()]];
		workspace.currentTabGroupIndex = 0;
		workspace.currentTabIndex = 0;
	}

	async function closeOtherTabGroups(keepTabGroup?: Tab[]) {
		const workspace = currentWorkspace.value;

		if (!workspace) {
			return;
		}

		const groupToKeep =
			keepTabGroup ?? workspace.tabGroups[workspace.currentTabGroupIndex] ?? workspace.tabGroups[0];

		if (!groupToKeep) {
			return;
		}

		const groupsToClose = workspace.tabGroups.filter(
			(group) => group[0]?.id !== groupToKeep[0]?.id
		);
		await Promise.all(groupsToClose.map((group) => closeTabGroupSessions(group)));

		workspace.tabGroups = [groupToKeep];
		workspace.currentTabGroupIndex = 0;
		workspace.currentTabIndex = 0;
	}

	function setCurrentTabGroupIndex(newTabGroupIndex: number) {
		const workspace = currentWorkspace.value;

		if (!workspace || workspace.tabGroups.length === 0) {
			return;
		}

		const index = Math.min(Math.max(0, newTabGroupIndex), workspace.tabGroups.length - 1);
		workspace.currentTabGroupIndex = index;
		workspace.currentTabIndex = 0;
	}

	async function addNewTabGroup() {
		const workspace = currentWorkspace.value;

		if (!workspace) {
			return null;
		}

		const newTabGroup = [createNewTerminalTab()];
		workspace.tabGroups.push(newTabGroup);
		return newTabGroup;
	}

	async function openNewTabGroup() {
		const newTabGroup = await addNewTabGroup();
		if (!newTabGroup) {
			return;
		}

		openTabGroup(newTabGroup);
	}

	async function openTabGroup(tabGroup: TabGroup) {
		const workspace = currentWorkspace.value;

		if (!workspace) {
			return;
		}

		const tabGroupIndex = workspace.tabGroups.findIndex(
			(group) => group[0]?.id === tabGroup[0]?.id
		);
		if (tabGroupIndex === -1) {
			return;
		}

		setCurrentTabGroupIndex(tabGroupIndex);
	}

	function setTabs(tabGroups: TabGroup[]) {
		const workspace = currentWorkspace.value;

		if (!workspace || tabGroups.length === 0) {
			return;
		}

		const currentTabId = currentTab.value?.id ?? '';
		workspace.tabGroups = tabGroups;

		const newCurrentTabGroupIndex =
			workspace.tabGroups.findIndex((group) => group.some((tab) => tab.id === currentTabId)) ?? -1;

		setCurrentTabGroupIndex(newCurrentTabGroupIndex === -1 ? 0 : newCurrentTabGroupIndex);
	}

	async function init() {
		ensureAtLeastOneTabGroup();
	}

	async function preloadDefaultTab() {
		ensureAtLeastOneTabGroup();
	}

	function setTabFilterQuery(tab: Tab, filterQuery: string) {
		tab.filterQuery = filterQuery;
	}

	function setTabRuntimeInfo(
		tabId: string,
		payload: {
			runtimeCwd?: string;
			runtimeCommand?: string;
		}
	) {
		for (const workspace of workspaces.value) {
			for (const group of workspace.tabGroups) {
				const tab = group.find((item) => item.id === tabId);
				if (!tab) {
					continue;
				}

				tab.runtimeCwd = payload.runtimeCwd;
				tab.runtimeCommand = payload.runtimeCommand;
				return;
			}
		}
	}

	function toggleSplitView() {
		return;
	}

	function setCurrentTabIndex(newTabIndex: number) {
		const workspace = currentWorkspace.value;

		if (!workspace) {
			return;
		}

		const currentGroup = workspace.tabGroups[workspace.currentTabGroupIndex];
		if (!currentGroup?.length) {
			workspace.currentTabIndex = 0;
			return;
		}

		const index = Math.min(Math.max(0, newTabIndex), currentGroup.length - 1);
		workspace.currentTabIndex = index;
	}

	async function getDirEntry() {
		return null;
	}

	return {
		workspaces,
		primaryWorkspace,
		currentWorkspace,
		tabs,
		currentTabGroup,
		currentTab,
		init,
		addNewTabGroup,
		openNewTabGroup,
		preloadDefaultTab,
		getDirEntry,
		openTabGroup,
		closeTabGroup,
		closeAllTabGroups,
		closeOtherTabGroups,
		setTabs,
		toggleSplitView,
		setTabFilterQuery,
		setTabRuntimeInfo,
		setCurrentTabIndex,
		setCurrentTabGroupIndex,
	};
});
