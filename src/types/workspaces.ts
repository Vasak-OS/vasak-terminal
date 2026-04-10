export type Workspace = {
	id: number;
	isPrimary: boolean;
	isCurrent: boolean;
	name: string;
	actions: TabAction[];
	tabGroups: TabGroup[];
	currentTabGroupIndex: number;
	currentTabIndex: number;
};

export type Tab = {
	id: string;
	name: string;
	path: string;
	runtimeCwd?: string;
	runtimeCommand?: string;
	type: 'directory' | 'file' | 'search';
	paneWidth: number;
	filterQuery: string;
};

export type TabGroup = Tab[];

export type TabAction = {
	name: string;
	path: string;
};
