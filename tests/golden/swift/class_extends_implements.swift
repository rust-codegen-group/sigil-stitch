import Foundation

import MyModule

class AdminService: BaseService, Codable, Hashable {
    func isAdmin() -> Bool {
        return true
    }
}
