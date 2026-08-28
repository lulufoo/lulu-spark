export type TreeNode = {
  name: string;
  relative_path: string;
  is_dir: boolean;
  expanded: boolean;
  loaded: boolean;
  children: TreeNode[];
};

export type RepoPickerOption = { value: string; label: string; title: string };

export type RepoRecord = { full_name: string };
