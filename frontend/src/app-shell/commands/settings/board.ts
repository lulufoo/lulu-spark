import { deleteBoardPat, fetchBoardPatStatus, setBoardPat } from '../../../host/api.ts';
import { SAVED_KEY_PLACEHOLDER } from '../../state/settings/store.ts';
import { patchBoardPat } from '../../state/settings/board-pat.ts';

const SAVE_OK = 'Board token saved.';
const SAVE_FAIL = "Couldn't save the Board token.";
const REMOVE_OK = 'Board token removed.';
const REMOVE_FAIL = "Couldn't remove the Board token.";
const STATUS_FAIL = "Couldn't read the Board token status.";
const NEED_TOKEN = 'Enter a Board personal access token.';

export async function loadBoardPatStatus() {
  patchBoardPat({ busy: true });
  try {
    const configured = (await fetchBoardPatStatus()) === true;
    patchBoardPat({ configured, busy: false });
  } catch {
    patchBoardPat({ configured: null, busy: false, message: STATUS_FAIL });
  }
}

export async function saveBoardPat(pat: string): Promise<boolean> {
  const value = pat.trim();
  if (!value || value === SAVED_KEY_PLACEHOLDER) {
    patchBoardPat({ message: NEED_TOKEN });
    return false;
  }
  patchBoardPat({ busy: true, message: '' });
  try {
    await setBoardPat(value);
    patchBoardPat({ configured: true, busy: false, message: SAVE_OK });
    return true;
  } catch {
    patchBoardPat({ busy: false, message: SAVE_FAIL });
    return false;
  }
}

export async function removeBoardPat(): Promise<boolean> {
  patchBoardPat({ busy: true, message: '' });
  try {
    await deleteBoardPat();
    patchBoardPat({ configured: false, busy: false, message: REMOVE_OK });
    return true;
  } catch {
    patchBoardPat({ busy: false, message: REMOVE_FAIL });
    return false;
  }
}
