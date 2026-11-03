import type { CaseLocation } from "../api/cases";

/** Only offer locations already included in the caller's authorized records. */
export function CaseLocationSelect({
  locations,
  value,
  onChange,
  label = "使用已有地点",
}: {
  locations: (CaseLocation | null | undefined)[];
  value: string;
  onChange: (location: CaseLocation | null) => void;
  label?: string;
}) {
  const unique = [...new Map(
    locations.filter((location): location is CaseLocation => !!location)
      .map((location) => [location.id, location]),
  ).values()];
  if (unique.length === 0) return null;
  return (
    <label className="block space-y-1 text-sm text-slate-700">
      <span>{label}</span>
      <select
        className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
        value={value}
        onChange={(event) => onChange(
          unique.find((location) => location.id === event.target.value) ?? null,
        )}
      >
        <option value="">填写新地点</option>
        {unique.map((location) => (
          <option key={location.id} value={location.id}>
            {location.location_text ?? `${location.longitude}, ${location.latitude}`}
          </option>
        ))}
      </select>
    </label>
  );
}
