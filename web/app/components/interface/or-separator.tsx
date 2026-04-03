export function OrSeparator() {
  return (
    <div className="flex flex-col gap-2 items-center justify-center relative my-5">
      <hr className="w-full absolute top-1/2 left-0 border-t border-neutral-700 -translate-y-1/2" />
      <p className="text-neutral-400 text-base bg-neutral-900 px-2 relative">
        or
      </p>
    </div>
  );
}
