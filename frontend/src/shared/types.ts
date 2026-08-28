export type CommentLike = {
  id?: string;
  text?: string;
};

export type LayerData = {
  comments?: CommentLike[];
};

export type Annotation = Record<string, unknown>;

export type WriteResult = {
  ok?: boolean;
  error?: string;
};

export type FloatingListSelectOption = {
  value: string;
  label: string;
  title?: string;
};

export type FloatingListSelectConfig = {
  ariaLabel: string;
  value: string;
  options: FloatingListSelectOption[];
  pickerClass?: string;
  onSelect: (value: string) => void;
};

export type FloatingListSelectSync = {
  value: string;
  options: FloatingListSelectOption[];
};

export type FloatingListOpenState = {
  menu: HTMLElement;
  trigger: HTMLElement;
  picker: HTMLElement;
  docListener: (event: MouseEvent) => void;
};

export type ListSelectPicker = HTMLElement & {
  _listSelectOptions?: FloatingListSelectOption[];
  _listSelectValue?: string;
};
