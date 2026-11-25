import { Hero } from "@/components/home/Hero";
import { ShowcaseStage } from "@/components/home/HeroShowcase";
import { DeviceLab } from "@/components/home/DeviceLab";
import { VsCode } from "@/components/home/VsCode";
import { Pillars } from "@/components/home/sections/Pillars";
import { HowItWorks } from "@/components/home/sections/HowItWorks";
import { Frameworks } from "@/components/home/sections/Frameworks";
import { Agents } from "@/components/home/sections/Agents";
import { ArchitectureSection } from "@/components/home/sections/ArchitectureSection";
import { Privacy } from "@/components/home/sections/Privacy";
import { OpenSource } from "@/components/home/sections/OpenSource";
import { Faq } from "@/components/home/sections/Faq";
import { Close } from "@/components/home/sections/Close";
import { loadTools } from "@/components/docs/McpToolTable";
import { latestStable, loadManifest } from "@/lib/releases";

export default function Home() {
  const tools = loadTools().map((t) => ({ name: t.name, annotations: { readOnlyHint: t.annotations.readOnlyHint } }));
  const hasRelease = latestStable(loadManifest()) !== null;
  return (
    <>
      <Hero hasRelease={hasRelease} showcase={<DeviceLab poster={<ShowcaseStage />} />} />
      <Pillars />
      <HowItWorks />
      <Frameworks />
      <Agents tools={tools} />
      <VsCode />
      <ArchitectureSection />
      <Privacy />
      <OpenSource />
      <Faq />
      <Close hasRelease={hasRelease} />
    </>
  );
}
