interface PlaceholderProps {
  label: string;
  description?: string;
}

export function Placeholder({ label, description }: PlaceholderProps) {
  return (
    <div class="panel-placeholder">
      <div class="placeholder-label">{label}</div>
      {description && <div class="placeholder-desc">{description}</div>}
    </div>
  );
}
