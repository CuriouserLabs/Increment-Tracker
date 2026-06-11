// Removable filter chips — filter state is always visible, never hidden
// behind a modal (spec §6).

export interface ChipOption {
  id: string;
  label: string;
  active: boolean;
}

interface Props {
  options: ChipOption[];
  onToggle: (id: string) => void;
}

export function FilterChips({ options, onToggle }: Props) {
  return (
    <div className="row" style={{ flexWrap: "wrap", gap: 8 }}>
      {options.map((o) => (
        <button
          key={o.id}
          className={`chip ${o.active ? "active" : ""}`}
          onClick={() => onToggle(o.id)}
        >
          {o.label}
          {o.active ? " ✕" : ""}
        </button>
      ))}
    </div>
  );
}
