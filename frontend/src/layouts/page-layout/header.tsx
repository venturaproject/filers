import { ThemeToggle } from "@/components/theme/theme-toggle";
import { Button } from "@/components/ui/button";
import { DocumentTitle } from "@/components/document-title";
import { APP_NAME } from "@/config/env";
import { ArrowRight } from "lucide-react";

export const Header = ({ title } : any) => {
  return (
    <>
      <DocumentTitle title={title ?? APP_NAME}/>
      <header className="container mx-auto flex h-fit items-center justify-between py-4">
      <h1 className="flex items-center gap-2 text-xl font-bold">
        <img
          src={"/avatars/shadcn.jpg"}
          width={30}
          height={30}
          className="select-none rounded border shadow-md"
          alt="shadcn/ui"
        />
        <span>{APP_NAME}</span>
      </h1>
      <nav className="flex items-center gap-4">
        <a
          target="_blank"
          href="https://github.com/binjuhor/shadcn-admin"
        >
          <Button
            variant="default"
            className="h-fit rounded-full bg-[#222] font-semibold text-white hover:bg-[#222]/90"
          >
            Github
            <ArrowRight className="ml-2 h-4 w-4" />
          </Button>
        </a>
        <ThemeToggle />
      </nav>
    </header>
    </>
  );
};
