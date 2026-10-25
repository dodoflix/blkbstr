import {
  createContext,
  forwardRef,
  useContext,
  useId,
  useRef,
  useState,
  type DragEvent,
  type InputHTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  type TextareaHTMLAttributes,
} from "react";
import { Checkbox as RCheckbox, RadioGroup as RRadio, Select as RSelect, Slider as RSlider, Switch as RSwitch, ToggleGroup } from "radix-ui";
import { cx } from "./cx";
import { Icon, type IconName } from "./Icon";
import { Chip, IconButton, Text } from "./primitives";
import type { Size } from "./types";

type FieldCtx = { id: string; hintId: string; errorId: string; invalid: boolean; hasHint: boolean };
const FieldContext = createContext<FieldCtx | null>(null);

/** Props every control should spread so it is labelled and described by its Field. */
function useFieldProps() {
  const f = useContext(FieldContext);
  if (!f) return {};
  return {
    id: f.id,
    "aria-invalid": f.invalid || undefined,
    "aria-describedby": [f.hasHint && f.hintId, f.invalid && f.errorId].filter(Boolean).join(" ") || undefined,
  };
}

/** Label, control, hint and error as one unit. Controls inside pick up id and aria wiring. */
export function Field({
  label,
  hint,
  error,
  optional,
  children,
  className,
}: {
  label?: ReactNode;
  hint?: ReactNode;
  error?: ReactNode;
  optional?: boolean;
  children: ReactNode;
  className?: string;
}) {
  const id = useId();
  const ctx: FieldCtx = { id, hintId: `${id}-hint`, errorId: `${id}-err`, invalid: Boolean(error), hasHint: Boolean(hint) };
  return (
    <FieldContext.Provider value={ctx}>
      <div className={cx("ui-field", className)}>
        {label && (
          <label className="ui-field__label" htmlFor={id}>
            {label}
            {optional && <span className="ui-field__opt">optional</span>}
          </label>
        )}
        {children}
        {hint && !error && (
          <Text as="p" size="sm" color={3} id={ctx.hintId}>
            {hint}
          </Text>
        )}
        {error && (
          <Text as="p" size="sm" color="danger" id={ctx.errorId} role="alert">
            {error}
          </Text>
        )}
      </div>
    </FieldContext.Provider>
  );
}

type TextFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "size" | "prefix"> & {
  size?: Size;
  icon?: IconName;
  suffix?: ReactNode;
  invalid?: boolean;
  mono?: boolean;
};

export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(function TextField(
  { size = "md", icon, suffix, invalid, mono, className, ...rest },
  ref,
) {
  const field = useFieldProps();
  return (
    <div className={cx("ui-input", className)} data-size={size} data-invalid={invalid || field["aria-invalid"] || undefined}>
      {icon && <Icon name={icon} size={16} />}
      <input ref={ref} className={cx(mono && "ui-input--mono")} {...field} {...rest} />
      {suffix && <span className="ui-input__suffix">{suffix}</span>}
    </div>
  );
});

export const SearchField = forwardRef<HTMLInputElement, Omit<TextFieldProps, "icon" | "type"> & { onClear?: () => void }>(
  function SearchField({ onClear, value, ...rest }, ref) {
    return (
      <TextField
        ref={ref}
        type="search"
        icon="search"
        value={value}
        suffix={
          onClear && value ? <IconButton icon="x" label="Clear" size="sm" onClick={onClear} /> : undefined
        }
        {...rest}
      />
    );
  },
);

export const TextArea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement> & { mono?: boolean; invalid?: boolean }>(
  function TextArea({ mono, invalid, className, rows = 4, ...rest }, ref) {
    const field = useFieldProps();
    return (
      <textarea
        ref={ref}
        rows={rows}
        className={cx("ui-textarea", mono && "ui-input--mono", className)}
        data-invalid={invalid || field["aria-invalid"] || undefined}
        {...field}
        {...rest}
      />
    );
  },
);

export function NumberField({
  value,
  onChange,
  min,
  max,
  step = 1,
  suffix,
  size = "md",
}: {
  value: number;
  onChange: (next: number) => void;
  min?: number;
  max?: number;
  step?: number;
  suffix?: ReactNode;
  size?: Size;
}) {
  const field = useFieldProps();
  const clamp = (n: number) => Math.min(max ?? Infinity, Math.max(min ?? -Infinity, n));
  return (
    <div className="ui-input ui-number" data-size={size}>
      <IconButton icon="minus" label="Decrease" size="sm" onClick={() => onChange(clamp(value - step))} disabled={min !== undefined && value <= min} />
      <input
        type="number"
        value={Number.isFinite(value) ? value : ""}
        min={min}
        max={max}
        step={step}
        onChange={(e) => onChange(clamp(e.currentTarget.valueAsNumber))}
        {...field}
      />
      {suffix && <span className="ui-input__suffix">{suffix}</span>}
      <IconButton icon="plus" label="Increase" size="sm" onClick={() => onChange(clamp(value + step))} disabled={max !== undefined && value >= max} />
    </div>
  );
}

export type Option<V extends string = string> = {
  value: V;
  label: ReactNode;
  description?: ReactNode;
  icon?: IconName;
  disabled?: boolean;
};

export function Select<V extends string>({
  value,
  onValueChange,
  options,
  placeholder = "Select...",
  size = "md",
  disabled,
  "aria-label": ariaLabel,
}: {
  value?: V;
  onValueChange: (value: V) => void;
  options: Option<V>[];
  placeholder?: string;
  size?: Size;
  disabled?: boolean;
  "aria-label"?: string;
}) {
  const field = useFieldProps();
  return (
    <RSelect.Root value={value} onValueChange={(v) => onValueChange(v as V)} disabled={disabled}>
      <RSelect.Trigger className="ui-select" data-size={size} aria-label={ariaLabel} {...field}>
        <RSelect.Value placeholder={placeholder} />
        <RSelect.Icon>
          <Icon name="chevrons-vertical" size={16} />
        </RSelect.Icon>
      </RSelect.Trigger>
      <RSelect.Portal>
        <RSelect.Content className="ui-menu" position="popper" sideOffset={6}>
          <RSelect.Viewport className="ui-menu__viewport">
            {options.map((o) => (
              <RSelect.Item key={o.value} value={o.value} disabled={o.disabled} className="ui-menu__item">
                {o.icon && <Icon name={o.icon} size={16} />}
                <span className="ui-menu__text">
                  <RSelect.ItemText>{o.label}</RSelect.ItemText>
                  {o.description && <span className="ui-menu__desc">{o.description}</span>}
                </span>
                <RSelect.ItemIndicator className="ui-menu__check">
                  <Icon name="check" size={16} />
                </RSelect.ItemIndicator>
              </RSelect.Item>
            ))}
          </RSelect.Viewport>
        </RSelect.Content>
      </RSelect.Portal>
    </RSelect.Root>
  );
}

export function Checkbox({
  checked,
  onCheckedChange,
  label,
  description,
  disabled,
}: {
  checked: boolean | "indeterminate";
  onCheckedChange: (next: boolean) => void;
  label: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
}) {
  const id = useId();
  return (
    <div className="ui-check">
      <RCheckbox.Root id={id} className="ui-check__box" checked={checked} disabled={disabled} onCheckedChange={(v) => onCheckedChange(v === true)}>
        <RCheckbox.Indicator>
          <Icon name={checked === "indeterminate" ? "minus" : "check"} size={14} strokeWidth={2.4} />
        </RCheckbox.Indicator>
      </RCheckbox.Root>
      <label htmlFor={id} className="ui-check__text">
        <span>{label}</span>
        {description && <Text size="sm" color={3}>{description}</Text>}
      </label>
    </div>
  );
}

export function Switch({
  checked,
  onCheckedChange,
  label,
  description,
  disabled,
  size = "md",
}: {
  checked: boolean;
  onCheckedChange: (next: boolean) => void;
  label: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
  size?: "sm" | "md";
}) {
  const id = useId();
  return (
    <div className="ui-switchrow">
      <label htmlFor={id} className="ui-check__text">
        <span>{label}</span>
        {description && <Text size="sm" color={3}>{description}</Text>}
      </label>
      <RSwitch.Root id={id} className="ui-switch" data-size={size} checked={checked} disabled={disabled} onCheckedChange={onCheckedChange}>
        <RSwitch.Thumb className="ui-switch__thumb" />
      </RSwitch.Root>
    </div>
  );
}

export type RadioOption<V extends string = string> = Option<V> & { content?: ReactNode };

/** `list` is a plain radio list; `cards` makes each option a larger target with room for content. */
export function RadioGroup<V extends string>({
  value,
  onValueChange,
  options,
  variant = "list",
  "aria-label": ariaLabel,
}: {
  value: V | undefined;
  onValueChange: (value: V) => void;
  options: RadioOption<V>[];
  variant?: "list" | "cards";
  "aria-label"?: string;
}) {
  const base = useId();
  return (
    <RRadio.Root className="ui-radios" data-variant={variant} value={value ?? ""} onValueChange={(v) => onValueChange(v as V)} aria-label={ariaLabel}>
      {options.map((o, i) => {
        const id = `${base}-${i}`;
        return (
          <div key={o.value} className="ui-radio" data-checked={value === o.value || undefined}>
            <RRadio.Item id={id} value={o.value} disabled={o.disabled} className="ui-radio__dot">
              <RRadio.Indicator className="ui-radio__fill" />
            </RRadio.Item>
            <label htmlFor={id} className="ui-radio__body">
              <span className="ui-radio__label">{o.label}</span>
              {o.description && <Text size="sm" color={3}>{o.description}</Text>}
              {o.content}
            </label>
          </div>
        );
      })}
    </RRadio.Root>
  );
}

export function SegmentedControl<V extends string>({
  value,
  onValueChange,
  options,
  size = "md",
  "aria-label": ariaLabel,
  iconOnly,
  fullWidth,
}: {
  value: V;
  onValueChange: (value: V) => void;
  options: Option<V>[];
  size?: Size;
  iconOnly?: boolean;
  fullWidth?: boolean;
  "aria-label": string;
}) {
  return (
    <ToggleGroup.Root
      type="single"
      className="ui-seg"
      data-size={size}
      data-full={fullWidth || undefined}
      value={value}
      aria-label={ariaLabel}
      // Radix reports "" when the active item is clicked again; a segmented control always has one.
      onValueChange={(v) => v && onValueChange(v as V)}
    >
      {options.map((o) => (
        <ToggleGroup.Item key={o.value} value={o.value} className="ui-seg__item" aria-label={iconOnly && typeof o.label === "string" ? o.label : undefined} title={iconOnly && typeof o.label === "string" ? o.label : undefined}>
          {o.icon && <Icon name={o.icon} size={size === "sm" ? 14 : 16} />}
          {!iconOnly && o.label}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}

export function Slider({
  value,
  onValueChange,
  min = 0,
  max = 100,
  step = 1,
  format = String,
  "aria-label": ariaLabel,
}: {
  value: number;
  onValueChange: (next: number) => void;
  min?: number;
  max?: number;
  step?: number;
  format?: (n: number) => string;
  "aria-label": string;
}) {
  return (
    <div className="ui-slider">
      <RSlider.Root className="ui-slider__root" value={[value]} min={min} max={max} step={step} onValueChange={([v]) => onValueChange(v)}>
        <RSlider.Track className="ui-slider__track">
          <RSlider.Range className="ui-slider__range" />
        </RSlider.Track>
        <RSlider.Thumb className="ui-slider__thumb" aria-label={ariaLabel} />
      </RSlider.Root>
      <Text size="sm" color={2} mono className="ui-slider__value">
        {format(value)}
      </Text>
    </div>
  );
}

export function TagInput({
  value,
  onChange,
  placeholder = "Add and press Enter",
  validate,
}: {
  value: string[];
  onChange: (next: string[]) => void;
  placeholder?: string;
  /** Return an error string to reject the tag. */
  validate?: (tag: string) => string | undefined;
}) {
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string>();
  const field = useFieldProps();
  const commit = () => {
    const tag = draft.trim().replace(/,$/, "");
    if (!tag) return;
    const problem = validate?.(tag) ?? (value.includes(tag) ? "Already added" : undefined);
    setError(problem);
    if (problem) return;
    onChange([...value, tag]);
    setDraft("");
  };
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      commit();
    } else if (e.key === "Backspace" && !draft && value.length) {
      onChange(value.slice(0, -1));
    }
  };
  return (
    <div className="ui-input ui-tags" data-invalid={error ? true : undefined}>
      {value.map((tag) => (
        <Chip key={tag} onRemove={() => onChange(value.filter((t) => t !== tag))}>
          {tag}
        </Chip>
      ))}
      <input value={draft} placeholder={value.length ? "" : placeholder} onChange={(e) => setDraft(e.currentTarget.value)} onKeyDown={onKey} onBlur={commit} {...field} />
      {error && (
        <Text size="xs" color="danger" role="alert">
          {error}
        </Text>
      )}
    </div>
  );
}

/** Click or drop. Reads nothing itself; hands the File objects to the caller. */
export function FileDrop({
  onFiles,
  accept,
  multiple,
  title = "Drop a file here",
  hint = "or click to choose",
}: {
  onFiles: (files: File[]) => void;
  accept?: string;
  multiple?: boolean;
  title?: ReactNode;
  hint?: ReactNode;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [over, setOver] = useState(false);
  const drop = (e: DragEvent) => {
    e.preventDefault();
    setOver(false);
    const files = [...e.dataTransfer.files];
    if (files.length) onFiles(multiple ? files : files.slice(0, 1));
  };
  return (
    <button
      type="button"
      className="ui-drop"
      data-over={over || undefined}
      onClick={() => input.current?.click()}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={drop}
    >
      <Icon name="upload" size={22} />
      <span className="ui-drop__title">{title}</span>
      <Text size="sm" color={3}>{hint}</Text>
      <input
        ref={input}
        type="file"
        hidden
        accept={accept}
        multiple={multiple}
        onChange={(e) => {
          const files = [...(e.currentTarget.files ?? [])];
          if (files.length) onFiles(files);
          e.currentTarget.value = "";
        }}
      />
    </button>
  );
}

export type Choice<V extends string = string> = { value: V; label: ReactNode; description?: ReactNode; icon?: IconName };

/** One-of-N as a grid of larger tiles; for choices that deserve a sentence each. */
export function ChoiceGrid<V extends string>({
  value,
  onValueChange,
  choices,
  min = "9rem",
  "aria-label": ariaLabel,
}: {
  value: V;
  onValueChange: (value: V) => void;
  choices: Choice<V>[];
  min?: string;
  "aria-label": string;
}) {
  return (
    <RRadio.Root
      className="ui-choices"
      style={{ gridTemplateColumns: `repeat(auto-fit, minmax(min(${min}, 100%), 1fr))` }}
      value={value}
      onValueChange={(v) => onValueChange(v as V)}
      aria-label={ariaLabel}
    >
      {choices.map((c) => (
        <RRadio.Item key={c.value} value={c.value} className="ui-choice">
          {c.icon && <Icon name={c.icon} size={20} />}
          <span className="ui-choice__label">{c.label}</span>
          {c.description && <span className="ui-choice__desc">{c.description}</span>}
        </RRadio.Item>
      ))}
    </RRadio.Root>
  );
}

/** Colour chooser. `colors` maps each value to the swatch it should show. */
export function SwatchPicker<V extends string>({
  value,
  onValueChange,
  colors,
  "aria-label": ariaLabel,
}: {
  value: V;
  onValueChange: (value: V) => void;
  colors: Record<V, string>;
  "aria-label": string;
}) {
  return (
    <RRadio.Root className="ui-swatches" value={value} onValueChange={(v) => onValueChange(v as V)} aria-label={ariaLabel}>
      {(Object.keys(colors) as V[]).map((name) => (
        <RRadio.Item key={name} value={name} className="ui-swatch" style={{ background: colors[name] }} aria-label={name} title={name}>
          <RRadio.Indicator className="ui-swatch__tick">
            <Icon name="check" size={16} strokeWidth={2.6} />
          </RRadio.Indicator>
        </RRadio.Item>
      ))}
    </RRadio.Root>
  );
}
