import { faker } from "@faker-js/faker";
import type { Menu, Permission, Role, User } from "#/entity";
import { PermissionType } from "#/enum";

const { GROUP, MENU } = PermissionType;

export const DB_MENU: Menu[] = [
  // Single group with only API Key management
  { id: "group_api", name: "API Key", code: "api", parentId: "", type: GROUP },
  {
    id: "api_keys",
    parentId: "group_api",
    name: "API Key 管理",
    code: "api:keys",
    type: MENU,
    path: "/api-keys",
    component: "/pages/api-keys",
  },
];

export const DB_USER: User[] = [
	{ id: "user_admin_id", username: "admin", password: "demo1234", avatar: faker.image.avatarGitHub(), email: "admin@slash.com" },
	{ id: "user_test_id", username: "test", password: "demo1234", avatar: faker.image.avatarGitHub(), email: "test@slash.com" },
	{ id: "user_guest_id", username: "guest", password: "demo1234", avatar: faker.image.avatarGitHub(), email: "guest@slash.com" },
];

export const DB_ROLE: Role[] = [
	{ id: "role_admin_id", name: "admin", code: "SUPER_ADMIN" },
	{ id: "role_test_id", name: "test", code: "TEST" },
];

export const DB_PERMISSION: Permission[] = [
	{ id: "permission_create", name: "permission-create", code: "permission:create" },
	{ id: "permission_read", name: "permission-read", code: "permission:read" },
	{ id: "permission_update", name: "permission-update", code: "permission:update" },
	{ id: "permission_delete", name: "permission-delete", code: "permission:delete" },
];

export const DB_USER_ROLE = [
	{ id: "user_admin_role_admin", userId: "user_admin_id", roleId: "role_admin_id" },
	{ id: "user_test_role_test", userId: "user_test_id", roleId: "role_test_id" },
];

export const DB_ROLE_PERMISSION = [
	{ id: faker.string.uuid(), roleId: "role_admin_id", permissionId: "permission_create" },
	{ id: faker.string.uuid(), roleId: "role_admin_id", permissionId: "permission_read" },
	{ id: faker.string.uuid(), roleId: "role_admin_id", permissionId: "permission_update" },
	{ id: faker.string.uuid(), roleId: "role_admin_id", permissionId: "permission_delete" },

	{ id: faker.string.uuid(), roleId: "role_test_id", permissionId: "permission_read" },
	{ id: faker.string.uuid(), roleId: "role_test_id", permissionId: "permission_update" },
];
