import { setupWorker } from "msw/browser";
import { mockTokenExpired } from "./handlers/_demo";
import { menuList } from "./handlers/_menu";
import { signIn, signUp, userList } from "./handlers/_user";
import { listApiKeys, createApiKey, deleteApiKey } from "./handlers/_api_keys";

const handlers = [
  signIn,
  signUp,
  userList,
  mockTokenExpired,
  menuList,
  // admin secure endpoints
  listApiKeys,
  createApiKey,
  deleteApiKey,
];
const worker = setupWorker(...handlers);

export { worker };
