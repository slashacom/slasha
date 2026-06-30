type FormSectionProps = {
  title: string;
  description: string;
  children: React.ReactNode;
};

export function FormSection(props: FormSectionProps) {
  const { title, description, children } = props;

  return (
    <section className="space-y-5">
      <div>
        <h3 className="text-xs font-medium text-text-tertiary">{title}</h3>
        <p className="mt-1 text-[11px] text-text-tertiary">{description}</p>
      </div>
      {children}
    </section>
  );
}
