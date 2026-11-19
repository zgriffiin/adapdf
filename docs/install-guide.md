# Adapdf: install guide

This guide is for the staff who make accessible PDFs from Word guides. You need
no admin rights and no IT help to install Adapdf.

## What Adapdf is

Adapdf checks a Word guide for accessibility problems. It fixes what it safely
can. It then makes a tagged PDF through your own Word, checks the PDF, and writes
a report. The report is your evidence for the file. A person still has to check
the PDF in PAC and sign off.

Adapdf does not send your documents anywhere. It does its work on your computer
and does not need a network connection for it. Adapdf's window turns off the
background services of WebView2 (the Microsoft component that draws the window).
In 13 test runs on one computer, no connection to another address was seen. The
test looks at the connections about once a second, so it can miss a short one.
Word and Windows manage their own updates and can still use the network.

## What you need

- Windows 11.
- Microsoft 365 Word, installed on your computer.
- About 250 MB of free disk space (Adapdf and the data that its window keeps).
- The file `Adapdf_0.2.0_x64-setup.exe`. Your team lead gives you this file.

You do not need to install Java or any other program. Adapdf has what it needs.

## Install

1. Double-click `Adapdf_0.2.0_x64-setup.exe`.
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
3. Adapdf shows its findings. Under the heading it says which file it works on
   and where that file is. An error must be fixed before you can export.
   A warning does not stop the export. Each finding says where the problem is, in
   the words that Word uses (for example "In the page header of section 2 (all
   pages)").
4. Click **Fix problems** to let Adapdf fix what it can. It asks before each
   fix. **Adapdf never changes your original guide.** It saves the fixes in a
   copy, `accessible.docx`, in the folder `Guide - Adapdf`, and tells you: "Saved your
   fixes to accessible.docx. Your original Guide.docx was not changed." From then on
   Adapdf works on the copy. Adapdf never writes alt text or a title for you. For the
   title it shows buttons with texts from your guide (the file name, the Title
   paragraph, the first Heading 1). Nothing is filled in until you click one, and you can
   change the text. Type or click a text, or **Apply** stays off.
5. Some problems Adapdf cannot fix. Click **Open in Word** to open the
   copy in your own Word, make the change there, and save. Then click **Check
   again** in Adapdf. (If only your original is open in Adapdf, it asks to make
   the copy first, so that you change the copy and not your original.)
6. Click **Export PDF**. Wait. Word runs in the background. You do not see it.
   This takes about 10 seconds for a small guide, and more for a long one.
7. Adapdf shows **Pass** or **Fail**. Click **Open the report in your browser**
   to read the report.

If you open a guide that already has a copy, Adapdf asks: **Continue with the
accessible copy** (keeps the fixes and the changes that you made in Word) or **Start
over from the original**. Start over replaces the copy, including every change that
you made to it in Word, so Adapdf asks you to confirm it.

## Where the files go

Adapdf saves everything in one folder next to your Word guide. For `Guide.docx`
the folder is `Guide - Adapdf`. In it you get:

- `accessible.docx`: the copy with the fixes in it. Edit this file in Word.
- `Guide.pdf`: the PDF.
- `report.html`: the report. Open it in your browser.
- `report.json`: the same report for programs.

Your original `Guide.docx` is never changed, and Adapdf writes nothing else next to
it. There are no backup files: your original is the backup. Click **Open folder** in
Adapdf to open the folder.

Adapdf writes every fix into the copy, as a note that you can read in Word (File,
Info, Properties, Advanced Properties, Custom: **Adapdf fix log**). The report lists
the fixes. The note stays when you edit and save the copy in Word.

## If the path is too long

Word cannot open a file whose full path is longer than 259 characters. If the
folder of your guide is so deep that the path of `accessible.docx` would be longer,
**Fix problems** and **Open in Word** stop and say: "The path of the Word copy would
be N characters, and Word cannot open files with paths over 259 characters. Move the
guide to a folder with a shorter path (for example a folder nearer the top of
OneDrive), then try again." Nothing is written. Move the guide, then try again.
**Export PDF** does not have this limit.

## If Word has the file open

- **Fix problems** does not work while Word has the accessible copy open. Adapdf
  says: "Close the document in Word first, then try again." Adapdf does this to
  protect your work. Your original may stay open: Adapdf only reads it.
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
2. Open the Adapdf report (`report.html`) in your browser.
3. Look at the PDF for each item of the checklist.
4. Record the PAC result and the sign-off in the Adapdf app. In the app, on the
   result screen (or after **Open an earlier report**), find **Checklist and
   sign-off**. Choose the **PAC result** (Pass or Fail), type the **PAC
   version**, and click **Choose the PAC report file** to pick the PAC report. The
   file must be in the folder `Guide - Adapdf`. Tick each item that you checked.
   Type your name and pick the date. Click **Save sign-off**.

The report page in your browser does not have fields: it shows what you saved in the
app (for example "PAC: Pass, report ..., PAC 2024.1, recorded by ... on ..." and
"Signed by ... on ..."). Changes made in that page are not saved. Open the report in
the app to record them.

If PAC finds errors, fix the cause in the Word guide, not in the PDF. Then
export again. Exporting again makes a new, unsigned report.

## Uninstall

1. Open **Settings**, then **Apps**, then **Installed apps**.
2. Find **Adapdf**, click the three dots, and click **Uninstall**.
3. To also remove Adapdf's settings and the data that its window keeps (about
   50 MB), tick **Delete the application data** on the uninstall screen before
   you continue. If you do not tick it, that data stays on your computer. It is
   harmless, and a later install uses it again.

This removes the program, the Start menu entry and the folder
`%LOCALAPPDATA%\Adapdf`. It does not remove your guides, your accessible copies, your
PDFs or your reports.

## If something goes wrong

- **"The export did not finish."** The message says why. Check that Word is
  installed and starts when you open a guide yourself.
- **The window is blank, or the app does not start.** Windows 11 includes the
  part that draws the window (WebView2). If your computer has had it removed,
  ask your team lead.
- **Anything else.** Send your team lead the report and the text of the message.
