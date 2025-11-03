import { Icon } from "@/components/icon";
import type { NavProps } from "@/components/nav";

export const frontendNavData: NavProps["data"] = [
  {
    name: "API Key",
    items: [
      {
        title: "API Key 管理",
        path: "/api-keys",
        icon: <Icon icon="solar:key-bold-duotone" size={24} />,
      },
    ],
  },
];
