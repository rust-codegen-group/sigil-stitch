import com.example.auth.Authenticatable
import com.example.base.BaseService
import com.example.serial.Serializable

internal class AdminService : BaseService, Authenticatable, Serializable {
    fun isAdmin(): Boolean {
        return true
    }
}
