export function meta() {
  return [{ title: 'Account Settings · slasha' }];
}

export default function AccountSettings() {
  return (
    <div className="space-y-6">
      <div>
        <h3 className="font-semibold text-text">Account Settings</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Manage your account profile and security settings.
        </p>
      </div>
    </div>
  );
}
