import { cn } from "@/lib/cn";

/**
 * R4 section heading. `split` puts the heading on the left half and the lead on the right half
 * from 1024 px (editorial, left-aligned); `stack` keeps them in one column. The old eyebrow stays
 * only as the section's data label for scripts and analytics; nothing visible sits over a heading.
 */
export function SectionHead({
  id,
  label,
  title,
  lead,
  layout = "stack",
  className,
}: {
  id: string;
  label: string;
  title: string;
  lead?: string;
  layout?: "stack" | "split";
  className?: string;
}) {
  if (layout === "split") {
    return (
      <header className={cn("grid gap-6 lg:grid-cols-12 lg:items-end lg:gap-10", className)} data-reveal data-section={label}>
        <h2 id={id} className="t-section min-w-0 [overflow-wrap:break-word] lg:col-span-6">
          {title}
        </h2>
        {lead && <p className="t-lead max-w-[38rem] lg:col-span-5 lg:col-start-8">{lead}</p>}
      </header>
    );
  }
  return (
    <header className={cn("max-w-3xl", className)} data-reveal data-section={label}>
      <h2 id={id} className="t-section min-w-0 [overflow-wrap:break-word]">
        {title}
      </h2>
      {lead && <p className="t-lead mt-5">{lead}</p>}
    </header>
  );
}
