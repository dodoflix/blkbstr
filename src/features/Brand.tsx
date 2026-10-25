import { BrandMark } from "./BrandMark";

/** Logo and name; a button because it is also the way home. */
export function Brand({ onClick }: { onClick: () => void }) {
  return (
    <button type="button" className="app-brand" onClick={onClick} title="Home">
      <BrandMark size={22} />
      <span className="app-brand__word">Blockbuster</span>
    </button>
  );
}
