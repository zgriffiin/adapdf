# Adapdf: install guide

This guide is for the staff who make accessible PDFs from Word guides. You need
no admin rights and no IT help to install Adapdf.

## What Adapdf is

Adapdf checks a Word guide for accessibility problems. It fixes what it safely
can. It then makes a tagged PDF through your own Word, checks the PDF, and writes
a report. The report is your evidence for the file. A person still has to check
the PDF in PAC and sign off.

Everything stays on your computer. Adapdf sends nothing over the network.

## What you need

- Windows 11.
- Microsoft 365 Word, installed on your computer.
- About 200 MB of free disk space.
- The file `Adapdf_0.1.0_x64-setup.exe`. Your team lead gives you this file.

You do not need to install Java or any other program. Adapdf has what it needs.

## Install

1. Double-click `Adapdf_0.1.0_x64-setup.exe`.
2. Windows may show a blue box: "Windows protected your PC". The installer is
   not signed yet. If you got the file from your team lead, click **More info**,
   then **Run anyway**.
3. Follow the steps on the screen. Click **Install**. Wait about 10 seconds.
4. When the installer is done, click **Finish**.

Windows does not ask for an administrator password. If it does, stop and ask
your team lead.

Adapdf is now in your Start menu, under **Adapdf**. The files are in the folder
`%LOCALAPPDATA%\Adapdf`. This folder is only for your user account.

## First run

1. Open **Adapdf** from the Start menu.
2. Click **Choose a Word document**, or drop a `.docx` file on the window.
3. Adapdf shows its findings. An error must be fixed before you can export.
   A warning does not stop the export.
4. Click **Fix problems** to let Adapdf fix what it can. It asks before each
   fix. It makes a backup copy of your guide first. It never writes alt text or
   a title for you. It asks you to type these.
5. Click **Export PDF**. Wait. Word runs in the background. You do not see it.
   This takes about 10 seconds for a small guide, and more for a long one.
6. Adapdf shows **Pass** or **Fail**. Click **Open the report in your browser**
   to read the report.

## Where the files go

Adapdf saves all files next to your Word guide. For `Guide.docx` you get:

- `Guide.pdf`: the PDF.
- `Guide.accessibility.html`: the report. Open it in your browser.
- `Guide.accessibility.json`: the same report for programs.
- `Guide.backup-<date and time>.docx`: a backup of your guide. It is made only
  when you let Adapdf fix the guide.

Adapdf never changes the `.docx` file unless you click **Fix problems**.

## If Word has the file open

- **Fix problems** does not work while Word has the guide open. Adapdf tells
  you. Close the guide in Word, then try again. Adapdf does this to protect your
  work.
- **Export PDF** works while the guide is open, but it reads the file as it is
  saved on disk. Save the guide in Word first.
- If the PDF is open in a PDF viewer, Adapdf cannot replace it. Close the PDF,
  then export again.
- Adapdf starts its own hidden Word. It does not touch the Word windows that you
  have open.

## Get the PAC check done

Adapdf does not replace PAC. PAC is the checker of record. A person must run
it and write down the result.

1. Open the PDF in PAC, and run the check. Save the PAC report file next to the
   PDF.
2. Open the Adapdf report (`Guide.accessibility.html`).
3. Find the section **PAC**. Write the name of the PAC report file, the PAC
   version, and the result: **Pass** or **Fail**. The report page cannot save
   what you type in the PAC section yet. Keep the PAC report file next to the
   PDF, and print the page if you need a copy of these notes.
4. Do the checklist in the report. Look at the PDF for each item and tick it.
5. Open the report in the app (**Open an earlier report**), type your name and
   the date, and click **Save sign-off**.

If PAC finds errors, fix the cause in the Word guide, not in the PDF. Then
export again. Exporting again makes a new, unsigned report.

## Uninstall

1. Open **Settings**, then **Apps**, then **Installed apps**.
2. Find **Adapdf**, click the three dots, and click **Uninstall**.

This removes the program, the Start menu entry and the folder
`%LOCALAPPDATA%\Adapdf`. It does not remove your guides, your PDFs, your reports
or your backups.

## If something goes wrong

- **"The export did not finish."** The message says why. Check that Word is
  installed and starts when you open a guide yourself.
- **The window is blank, or the app does not start.** Windows 11 includes the
  part that draws the window (WebView2). If your computer has had it removed,
  ask your team lead.
- **Anything else.** Send your team lead the report and the text of the message.
