"use client";

import { useState, type FormEvent } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { ChoiceList } from "@/components/ChoiceList/ChoiceList";
import { Dialog } from "@/components/Dialog/Dialog";
import { Field } from "@/components/Field/Field";
import { Row, Stack } from "@/components/Layout/Layout";
import { Textarea } from "@/components/Textarea/Textarea";
import { useToast } from "@/components/Toast/Toast";
import { api, isApiError } from "@/lib/api/client";
import type { ReportReason, ReportTarget } from "@/lib/api/types";

const REASONS: Array<{ value: ReportReason; label: string; detail?: string; targets: ReportTarget[] }> = [
  { value: "spam", label: "Spam or advertising", targets: ["review", "place"] },
  { value: "offensive", label: "Offensive or harassing", targets: ["review", "place"] },
  { value: "not_a_student", label: "Not written by a student", targets: ["review"] },
  { value: "wrong_info", label: "Wrong information", detail: "Address, hours or deal", targets: ["place"] },
  { value: "other", label: "Something else", targets: ["review", "place"] },
];

interface ReportDialogProps {
  open: boolean;
  onClose: () => void;
  target: { type: ReportTarget; id: string; name: string } | null;
}

/** Small dialog to report a review or a place. */
export function ReportDialog({ open, onClose, target }: ReportDialogProps) {
  const toast = useToast();
  const [reason, setReason] = useState<ReportReason | null>(null);
  const [details, setDetails] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [reasonError, setReasonError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

  function close() {
    setReason(null);
    setDetails("");
    setError(null);
    setReasonError(null);
    onClose();
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!target) return;
    if (!reason) {
      setReasonError("Pick what's wrong.");
      return;
    }
    setSending(true);
    setError(null);
    try {
      await api.report({ target_type: target.type, target_id: target.id, reason, details: details.trim() || undefined });
      close();
      toast.show({ message: "Report sent. Thanks for keeping Corner useful." });
    } catch (err) {
      if (isApiError(err) && err.status === 401) setError("Log in with your school email to report.");
      else if (isApiError(err) && err.code === "not_verified") setError("Verify your school email to report.");
      else setError(isApiError(err) ? err.message : "Couldn't send the report. Try again.");
    } finally {
      setSending(false);
    }
  }

  const choices = REASONS.filter((r) => target && r.targets.includes(target.type));

  return (
    <Dialog
      open={open}
      onClose={close}
      title={target?.type === "place" ? "Report this place" : "Report this review"}
      description={target ? `Tell us what's wrong with ${target.type === "place" ? target.name : `${target.name}'s review`}. Reports are private.` : undefined}
    >
      <form onSubmit={submit} noValidate>
        <Stack gap={4}>
          {error ? <Alert tone="error">{error}</Alert> : null}
          <ChoiceList
            name="report-reason"
            label="Reason"
            choices={choices}
            value={reason}
            onChange={(v) => {
              setReason(v as ReportReason);
              setReasonError(null);
            }}
            error={reasonError}
          />
          <Field label="Anything else? (optional)">
            <Textarea rows={3} value={details} maxLength={500} onChange={(e) => setDetails(e.target.value)} />
          </Field>
          <Row gap={2} justify="flex-end">
            <Button variant="ghost" onClick={close}>
              Cancel
            </Button>
            <Button type="submit" loading={sending} loadingLabel="Sending…">
              Send report
            </Button>
          </Row>
        </Stack>
      </form>
    </Dialog>
  );
}
