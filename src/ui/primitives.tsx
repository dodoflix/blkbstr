import {
  forwardRef,
  type AnchorHTMLAttributes,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type ElementType,
  type HTMLAttributes,
  type ReactNode,
} from "react";
import { cx } from "./cx";
import { Icon, type IconName } from "./Icon";
import type { Size, Tone, Variant } from "./types";

export function Spinner({ size = 16, label }: { size?: number; label?: string }) {
  return (
    <svg
      className="ui-spinner"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      role={label ? "status" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      <rect x="3.5" y="3.5" width="17" height="17" rx="4" stroke="currentColor" strokeOpacity="0.25" strokeWidth="2.6" />
      <rect
        className="ui-spinner__arc"
        x="3.5"
        y="3.5"
        width="17"
        height="17"
        rx="4"
        stroke="currentColor"
        strokeWidth="2.6"
        strokeLinecap="round"
        pathLength="64"
        strokeDasharray="16 48"
      />
    </svg>
  );
}

type ButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> & {
  variant?: Variant;
  tone?: Tone;
  size?: Size;
  icon?: IconName;
  iconRight?: IconName;
  loading?: boolean;
  fullWidth?: boolean;
  children?: ReactNode;
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = "solid",
    tone = "accent",
    size = "md",
    icon,
    iconRight,
    loading,
    fullWidth,
    disabled,
    className,
    children,
    type = "button",
    ...rest
  },
  ref,
) {
  const iconSize = size === "sm" ? 15 : size === "lg" ? 20 : 17;
  return (
    <button
      ref={ref}
      type={type}
      className={cx("ui-btn", fullWidth && "ui-btn--full", className)}
      data-variant={variant}
      data-tone={tone}
      data-size={size}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      {...rest}
    >
      {loading ? <Spinner size={iconSize} /> : icon && <Icon name={icon} size={iconSize} />}
      {children !== undefined && children !== null && <span>{children}</span>}
      {iconRight && !loading && <Icon name={iconRight} size={iconSize} />}
    </button>
  );
});

type IconButtonProps = Omit<ButtonProps, "children" | "icon" | "iconRight" | "fullWidth"> & {
  icon: IconName;
  /** Required: an icon alone has no name for assistive tech. Also shown as the native tooltip. */
  label: string;
};

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { icon, label, variant = "ghost", tone = "neutral", size = "md", loading, className, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type="button"
      className={cx("ui-btn ui-btn--icon", className)}
      data-variant={variant}
      data-tone={tone}
      data-size={size}
      aria-label={label}
      title={label}
      aria-busy={loading || undefined}
      {...rest}
      disabled={rest.disabled || loading}
    >
      {loading ? <Spinner size={16} /> : <Icon name={icon} size={size === "sm" ? 15 : 18} />}
    </button>
  );
});

export function ButtonGroup({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cx("ui-btngroup", className)} role="group">
      {children}
    </div>
  );
}

export function Link({
  external,
  className,
  children,
  ...rest
}: AnchorHTMLAttributes<HTMLAnchorElement> & { external?: boolean }) {
  return (
    <a
      className={cx("ui-link", className)}
      {...(external ? { target: "_blank", rel: "noreferrer" } : null)}
      {...rest}
    >
      {children}
      {external && <Icon name="external" size={13} />}
    </a>
  );
}

type BadgeProps = {
  tone?: Tone;
  variant?: "soft" | "solid" | "outline";
  icon?: IconName;
  children: ReactNode;
  className?: string;
};

export function Badge({ tone = "neutral", variant = "soft", icon, children, className }: BadgeProps) {
  return (
    <span className={cx("ui-badge", className)} data-tone={tone} data-variant={variant}>
      {icon && <Icon name={icon} size={13} />}
      {children}
    </span>
  );
}

export function Chip({
  children,
  onRemove,
  tone = "neutral",
}: {
  children: ReactNode;
  onRemove?: () => void;
  tone?: Tone;
}) {
  return (
    <span className="ui-chip" data-tone={tone}>
      {children}
      {onRemove && (
        <button type="button" className="ui-chip__x" onClick={onRemove} aria-label="Remove">
          <Icon name="x" size={12} strokeWidth={2} />
        </button>
      )}
    </span>
  );
}

type TextProps<T extends ElementType> = {
  as?: T;
  size?: "xs" | "sm" | "md" | "lg" | "xl";
  /** 1 primary ink, 2 secondary, 3 tertiary, or a semantic tone. */
  color?: 1 | 2 | 3 | Tone;
  weight?: "regular" | "medium" | "semibold" | "bold";
  mono?: boolean;
  truncate?: boolean;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
} & Omit<HTMLAttributes<HTMLElement>, "color">;

export function Text<T extends ElementType = "span">({
  as,
  size = "md",
  color = 1,
  weight = "regular",
  mono,
  truncate,
  className,
  children,
  ...rest
}: TextProps<T>) {
  const Tag: ElementType = as ?? "span";
  const toned = typeof color === "string";
  return (
    <Tag
      className={cx("ui-text", mono && "ui-text--mono", truncate && "ui-text--truncate", className)}
      data-size={size}
      data-weight={weight}
      data-ink={toned ? undefined : color}
      data-tone={toned ? color : undefined}
      {...rest}
    >
      {children}
    </Tag>
  );
}

export function Heading({
  level = 2,
  size = "xl",
  className,
  children,
  ...rest
}: {
  level?: 1 | 2 | 3 | 4;
  size?: "md" | "lg" | "xl" | "2xl" | "3xl";
  className?: string;
  children?: ReactNode;
} & HTMLAttributes<HTMLHeadingElement>) {
  const Tag = `h${level}` as ElementType;
  return (
    <Tag className={cx("ui-heading", className)} data-size={size} {...rest}>
      {children}
    </Tag>
  );
}

export function Divider({
  vertical,
  className,
}: {
  vertical?: boolean;
  className?: string;
}) {
  return (
    <div
      role="separator"
      aria-orientation={vertical ? "vertical" : "horizontal"}
      className={cx("ui-divider", vertical && "ui-divider--v", className)}
    />
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="ui-kbd">{children}</kbd>;
}

export function Skeleton({
  width,
  height = 14,
  className,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
}) {
  return <span className={cx("ui-skeleton", className)} style={{ width, height }} aria-hidden />;
}

export function VisuallyHidden({ children }: { children: ReactNode }) {
  return <span className="ui-sr-only">{children}</span>;
}
