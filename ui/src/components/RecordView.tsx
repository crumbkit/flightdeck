import type { Row } from "@/lib/table";
import { CellValue } from "./CellValue";

/** Every field of one record, each name beside its value. */
export function RecordView({ record, label }: { record: Row; label: string }) {
  return (
    <dl aria-label={label} className="m-0 flex flex-col gap-1">
      {Object.entries(record).map(([name, value]) => (
        <div key={name} className="flex gap-4">
          <dt className="text-faint-foreground w-40 shrink-0 font-mono text-xs leading-5">
            {name}
          </dt>
          <dd className="m-0 min-w-0">
            <CellValue value={value} />
          </dd>
        </div>
      ))}
    </dl>
  );
}
