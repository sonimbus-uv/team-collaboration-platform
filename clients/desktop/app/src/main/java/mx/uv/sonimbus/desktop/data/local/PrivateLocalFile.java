package mx.uv.sonimbus.desktop.data.local;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.AclEntry;
import java.nio.file.attribute.AclEntryPermission;
import java.nio.file.attribute.AclEntryType;
import java.nio.file.attribute.AclFileAttributeView;
import java.nio.file.attribute.PosixFilePermissions;
import java.util.EnumSet;
import java.util.List;

public final class PrivateLocalFile {
    private PrivateLocalFile() {}

    public static void create(Path path) throws IOException {
        if (Files.getFileStore(path.getParent()).supportsFileAttributeView("posix")) {
            Files.createFile(
                    path,
                    PosixFilePermissions.asFileAttribute(
                            PosixFilePermissions.fromString("rw-------")));
            return;
        }
        Files.createFile(path);
        try {
            AclFileAttributeView acl = Files.getFileAttributeView(path, AclFileAttributeView.class);
            if (acl == null) {
                throw new IOException("Private local storage unavailable");
            }
            acl.setAcl(
                    List.of(
                            AclEntry.newBuilder()
                                    .setType(AclEntryType.ALLOW)
                                    .setPrincipal(Files.getOwner(path))
                                    .setPermissions(EnumSet.allOf(AclEntryPermission.class))
                                    .build()));
        } catch (IOException | RuntimeException failure) {
            Files.deleteIfExists(path);
            throw failure;
        }
    }
}
