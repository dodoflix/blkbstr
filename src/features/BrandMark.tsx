/** The app mark without its badge. The blocks follow the ink colour; the escaping block stays orange. */
export function BrandMark({ size = 22 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="14 14 204 204" fill="none" aria-hidden focusable="false">
      <defs>
        <linearGradient id="bm-burst" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#FF9466" />
          <stop offset="100%" stopColor="#E85A2A" />
        </linearGradient>
      </defs>
      <g stroke="#FF7A45" strokeLinecap="round">
        <line x1="148" y1="102" x2="158" y2="72" strokeWidth="7" opacity="0.25" />
        <line x1="163" y1="106" x2="171" y2="76" strokeWidth="6" opacity="0.4" />
        <line x1="178" y1="108" x2="184" y2="80" strokeWidth="5" opacity="0.55" />
      </g>
      <rect x="134" y="54" width="68" height="68" rx="12" stroke="var(--ink-3)" strokeWidth="4" strokeDasharray="7 7" opacity="0.7" />
      <g fill="var(--ink-1)">
        <rect x="54" y="54" width="68" height="68" rx="12" />
        <rect x="54" y="134" width="68" height="68" rx="12" />
        <rect x="134" y="134" width="68" height="68" rx="12" />
      </g>
      <rect x="122" y="118" width="12" height="12" rx="3" fill="#FF7A45" opacity="0.8" transform="rotate(24 128 124)" />
      <g transform="rotate(-18 184 52)">
        <rect x="150" y="18" width="68" height="68" rx="12" fill="url(#bm-burst)" />
        <path d="M172 18 L180 39 L164 53 L184 86" stroke="#B8431B" strokeWidth="4" strokeLinecap="round" strokeLinejoin="round" />
      </g>
    </svg>
  );
}
