import { Icon } from "@/components/icon";
import type { UploadProps } from "antd";
import Dragger from "antd/es/upload/Dragger";
import type { ReactElement } from "react";
import { StyledUploadBox } from "./styles";
import { getAuthHeaders } from "@/api/authHeaders";

interface Props extends UploadProps {
    placeholder?: ReactElement;
    /** default true: 自动注入 Authorization 与 CSRF 头 */
    withAuth?: boolean;
}
export function UploadBox({ placeholder, withAuth = true, ...other }: Props) {
    const headers = withAuth ? getAuthHeaders(other.headers) : other.headers;
    return (
        <StyledUploadBox>
            <Dragger {...other} headers={headers} showUploadList={false}>
                <div className="opacity-60 hover:opacity-50">
                    {placeholder || (
                        <div className="mx-auto flex h-16 w-16 items-center justify-center">
                            <Icon icon="eva:cloud-upload-fill" size={28} />
                        </div>
					)}
				</div>
			</Dragger>
		</StyledUploadBox>
	);
}
