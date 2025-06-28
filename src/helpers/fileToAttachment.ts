export const fileToAttachment = (
  item: DataTransferItem,
  saveAttachment: Function
) => {
  if (item.kind === "file") {
    const file = item.getAsFile();
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        if (e.target) {
          saveAttachment({
            type: file.type,
            content: e.target.result as string,
            filename: file.name,
          });
        }
      };
      reader.readAsDataURL(file);
    }
  } else if (item.type.includes("text/html")) {
    item.getAsString(async (html) => {
      const tempDiv = document.createElement("div");
      tempDiv.innerHTML = html;
      const img = tempDiv.querySelector("img");
      if (!img?.src) return;

      const imgSrc = img.src;
      if (imgSrc.startsWith("data:")) {
        const mimeType =
          imgSrc.substring(imgSrc.indexOf(":") + 1, imgSrc.indexOf(";")) || "";
        saveAttachment({
          type: mimeType,
          content: imgSrc,
          filename: "pasted_image.png",
        });
        return;
      }

      if (imgSrc.startsWith("blob:") || imgSrc.startsWith("http")) {
        saveAttachment({
          type: imgSrc.substring(imgSrc.lastIndexOf(".") + 1),
          content: imgSrc,
          filename: `pasted_image${imgSrc.substring(imgSrc.lastIndexOf("."))}`,
        });
      }
    });
  }
};
