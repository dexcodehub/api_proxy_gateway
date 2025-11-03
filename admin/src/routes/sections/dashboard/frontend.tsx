import type { RouteObject } from "react-router";
import { Component } from "./utils";

export function getFrontendDashboardRoutes(): RouteObject[] {
    const frontendDashboardRoutes: RouteObject[] = [
        { path: "api-keys", element: Component("/pages/api-keys") },
    ];
    return frontendDashboardRoutes;
}
