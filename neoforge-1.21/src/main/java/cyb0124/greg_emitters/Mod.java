package cyb0124.greg_emitters;

import com.sun.jna.*;
import com.sun.jna.platform.linux.Mman;
import com.sun.jna.platform.unix.LibCUtil;
import com.sun.jna.platform.win32.*;

import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.util.Map;

@net.neoforged.fml.common.Mod("greg_emitters")
public class Mod {
    public Mod() {
        String path;
        if (Platform.ARCH.equals("x86-64")) path = "/x64.bin";
        else if (Platform.ARCH.equals("aarch64")) path = "/aarch64.bin";
        else throw new UnsupportedOperationException("Unsupported architecture: " + Platform.ARCH);
        byte[] blob;
        try (InputStream is = Mod.class.getResourceAsStream(path)) {
            blob = is.readAllBytes();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
        ByteBuffer buf = ByteBuffer.wrap(blob, blob.length - 4, 4);
        int len = buf.getInt();
        Pointer mem = Platform.isWindows()
                ? Kernel32.INSTANCE.VirtualAllocEx(WinBase.INVALID_HANDLE_VALUE, null, new BaseTSD.SIZE_T(len), WinNT.MEM_COMMIT, WinNT.PAGE_READWRITE)
                : LibCUtil.mmap(null, len, Mman.PROT_READ | Mman.PROT_WRITE, Mman.MAP_PRIVATE | Mman.MAP_ANON, -1, 0);
        mem.write(0, blob, 0, len);
        buf.position(len);
        while (buf.remaining() > 12) {
            mem.setLong(buf.getInt(), Pointer.nativeValue(mem) + buf.getInt());
        }
        long exec_len = (long) buf.getInt() * 4096;
        if (Platform.isWindows()) {
            NativeLibrary.getInstance("kernel32").getFunction("VirtualProtect").invoke(new Object[]{mem, exec_len, WinNT.PAGE_EXECUTE_READ, new WinDef.DWORDByReference()});
            mem = mem.share(buf.getInt());
        } else {
            NativeLibrary.getInstance("c").getFunction("mprotect").invoke(new Object[]{mem, exec_len, Mman.PROT_READ | Mman.PROT_EXEC});
        }
        Function.getFunction(mem).invoke(Void.class, new Object[]{JNIEnv.CURRENT, this}, Map.of(Library.OPTION_ALLOW_OBJECTS, true));
    }
}
